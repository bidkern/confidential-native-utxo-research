"""Local-only two-node confidential-UTXO Bitcoin Core laboratory.

Usage: python integration/lab.py demo|start|stop|status|recover [alice|bob]
       python integration/lab.py pay alice bob 1000000
Wallet amounts and seeds are test data. No public-network connections are made.
"""
from __future__ import annotations
import base64, hashlib, json, os, re, socket, struct, subprocess, sys, time, urllib.request
from decimal import Decimal
from pathlib import Path

ROOT=Path(__file__).resolve().parents[1]
LAB=ROOT/'integration'; RUNS=LAB/'runs'; LATEST=LAB/'latest.json'
DAEMON=ROOT/'bitcoin-core/build/bin/Release/bitcoind.exe'
WALLET=ROOT/'target/release/cnu-wallet.exe'
SAT=100_000_000

class RPCError(Exception):
    def __init__(self,error): self.error=error; super().__init__(str(error))

def save(path,obj):
    path.write_text(json.dumps(obj,indent=2)+'\n')

def wallet(command,**kw):
    p=subprocess.run([str(WALLET)],input=json.dumps({'command':command,**kw}),text=True,capture_output=True,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
    if p.returncode: raise RuntimeError('Wallet helper failed: '+p.stderr[-2000:])
    return json.loads(p.stdout)

def rpc(node,method,*params,timeout=60):
    cookie=(Path(node['datadir'])/'regtest/.cookie').read_text().strip()
    req=urllib.request.Request(f"http://127.0.0.1:{node['rpc_port']}",data=json.dumps({'jsonrpc':'2.0','id':'cnu','method':method,'params':params}).encode(),headers={'Authorization':'Basic '+base64.b64encode(cookie.encode()).decode(),'Content-Type':'application/json'})
    try:
        with urllib.request.urlopen(req,timeout=timeout) as r: result=json.load(r)
    except urllib.error.HTTPError as e:
        result=json.load(e)
    if result.get('error'): raise RPCError(result['error'])
    return result['result']

def wait(fn,description,timeout=90):
    end=time.monotonic()+timeout; last=None
    while time.monotonic()<end:
        try:
            result=fn()
            if result: return result
        except (OSError,RPCError,ValueError) as e: last=e
        time.sleep(.2)
    raise RuntimeError(f'Timed out: {description}; last error={last}')

def free_ports(n):
    sockets=[]
    try:
        for _ in range(n):
            s=socket.socket();s.bind(('127.0.0.1',0));sockets.append(s)
        return [s.getsockname()[1] for s in sockets]
    finally:
        for s in sockets:s.close()

def load():
    if not LATEST.exists(): raise RuntimeError('Run the demo first.')
    return json.loads(LATEST.read_text())

def seed(state,name): return json.loads((Path(state['run'])/f'{name}.seed.json').read_text())['seed']
def identity(state,name): return wallet('identity',seed=seed(state,name))

def start_node(node,extra=()):
    try:
        rpc(node,'getblockcount',timeout=1)
        return
    except (OSError,RPCError,ValueError): pass
    datadir=Path(node['datadir']);datadir.mkdir(parents=True,exist_ok=True)
    args=[str(DAEMON),'-regtest','-server=1',f'-datadir={datadir}',f"-rpcport={node['rpc_port']}",'-rpcbind=127.0.0.1','-rpcallowip=127.0.0.1',f"-port={node['p2p_port']}",f"-bind=127.0.0.1:{node['p2p_port']}",'-connect=0','-dnsseed=0','-fixedseeds=0','-discover=0','-listenonion=0','-natpmp=0','-txindex=1','-acceptnonstdtxn=1','-minrelaytxfee=0.00000001','-blockmintxfee=0.00000001','-persistmempool=0','-assumevalid=0','-checkblocks=0','-checklevel=4','-uacomment=CNU-native-regtest','-printtoconsole=1',*extra]
    with (datadir/'console.log').open('ab') as log:
        proc=subprocess.Popen(args,stdin=subprocess.DEVNULL,stdout=log,stderr=subprocess.STDOUT,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
    node['pid']=proc.pid
    try:wait(lambda:rpc(node,'getblockchaininfo'),'RPC startup',180)
    except Exception:
        raise RuntimeError((datadir/'console.log').read_text(errors='replace')[-4000:])

def connect(state):
    a,b=state['nodes']
    for n in (a,b):rpc(n,'setnetworkactive',True)
    if not rpc(a,'getpeerinfo'):rpc(a,'addnode',f"127.0.0.1:{b['p2p_port']}",'onetry')
    wait(lambda:rpc(a,'getconnectioncount')>0 and rpc(b,'getconnectioncount')>0,'peer connection')

def sync(state):
    a,b=state['nodes'];return wait(lambda:rpc(a,'getbestblockhash')==rpc(b,'getbestblockhash'),'block synchronization')

def start(state):
    for n in state['nodes']:start_node(n)
    connect(state);sync(state);save(LATEST,state)

def stop_node(node):
    try:rpc(node,'stop')
    except (OSError,RPCError,ValueError):return
    def stopped():
        try:rpc(node,'getblockcount',timeout=1);return False
        except (OSError,RPCError,ValueError):return True
    wait(stopped,'node shutdown',60)
    # The RPC listener closes slightly before the process flushes chainstate.
    time.sleep(1)

def mine(state,count=1,node=0):
    hashes=rpc(state['nodes'][node],'generatetoaddress',count,identity(state,'alice')['mining_address'])
    return hashes

def public_output(o):
    script=bytes.fromhex(o['scriptPubKey']['hex'])
    if len(script)!=34 or script[1]!=32:return None
    if 'cnu_commitment' in o:
        assert script[0]==0x52
        return (b'\x01'+script[2:]+bytes.fromhex(o['cnu_commitment'])+bytes.fromhex(o['cnu_encrypted'])).hex()
    if script[0]!=0x51:return None
    value=int(Decimal(str(o['value']))*SAT)
    return (b'\x00'+struct.pack('<Q',value)+script[2:]).hex()

def recover_all(state,node_index=0,write=True):
    node=state['nodes'][node_index];history={}; wallets={'alice':{},'bob':{}}
    height=rpc(node,'getblockcount');tip=rpc(node,'getbestblockhash')
    for h in range(1,height+1):
        block=rpc(node,'getblock',rpc(node,'getblockhash',h),2)
        for tx in block['tx']:
            if 'cnu_payload' in tx:
                previous=[history[(i['txid'],i['vout'])] for i in tx['vin']]
                for name in wallets:
                    found=wallet('scan',seed=seed(state,name),payload=tx['cnu_payload'],previous=previous)['coins']
                    for coin in found:
                        assert coin['txid']==tx['txid'], 'native/payload transaction identity mismatch'
                        wallets[name][(coin['txid'],coin['vout'])]=coin
            for o in tx['vout']:
                encoded=public_output(o)
                if encoded is not None:history[(tx['txid'],o['n'])]=encoded
            for i in tx['vin']:
                if 'txid' in i:
                    op=(i['txid'],i['vout']);history.pop(op,None)
                    for w in wallets.values():w.pop(op,None)
    assert rpc(node,'getbestblockhash')==tip,'chain changed during recovery'
    result={name:list(coins.values()) for name,coins in wallets.items()}
    if write:
        for name,coins in result.items():save(Path(state['run'])/f'{name}.wallet.json',{'tip':tip,'coins':coins})
    return result

def balances(wallets):return {n:sum(c['value'] for c in coins) for n,coins in wallets.items()}

def broadcast_and_mine(state,tx):
    for n in state['nodes']:
        acceptance=rpc(n,'testmempoolaccept',[tx['hex']])[0]
        assert acceptance.get('allowed'),acceptance
    actual=rpc(state['nodes'][0],'sendrawtransaction',tx['hex'],0)
    assert actual==tx['txid']
    wait(lambda:tx['txid'] in rpc(state['nodes'][1],'getrawmempool'),'transaction relay')
    hashes=mine(state);sync(state)
    assert tx['txid'] in [t['txid'] for t in rpc(state['nodes'][1],'getblock',hashes[0],2)['tx']]
    return hashes[0]

def prepare_payment(state,sender,receiver,value,coins=None):
    if value<=0:raise ValueError('amount must be positive satoshis')
    coins=coins or recover_all(state)[sender]
    chosen=[];total=0
    for coin in sorted(coins,key=lambda c:c['value'],reverse=True):
        chosen.append(coin);total+=coin['value']
        if total>=value+1000:break
    if total<value+1000:raise ValueError('Insufficient confirmed confidential funds, including the 1,000-sat test fee')
    payments=[{'address':identity(state,receiver)['address'],'value':value}]
    change=total-value-1000
    if change:payments.append({'address':identity(state,sender)['address'],'value':change})
    return wallet('build',inputs=chosen,payments=payments,fee=1000)

def sha256d(b):return hashlib.sha256(hashlib.sha256(b).digest()).digest()
def compact(n):
    if n<253:return bytes([n])
    if n<=65535:return b'\xfd'+struct.pack('<H',n)
    return b'\xfe'+struct.pack('<I',n)
def bad_block(state,txhex):
    """Real solved regtest block with an adversarial transaction. No simulator validation."""
    n=state['nodes'][0];tip=rpc(n,'getbestblockhash');header=rpc(n,'getblockheader',tip);height=header['height']+1
    hbytes=height.to_bytes((height.bit_length()+7)//8,'little')
    if hbytes[-1]&128:hbytes+=b'\x00'
    script=bytes([len(hbytes)])+hbytes+b'\x00'
    owner=bytes.fromhex(identity(state,'alice')['owner'])
    # Regtest's subsidy interval is 150 blocks. Underclaiming transaction fees is valid.
    subsidy=(50*SAT)>>(height//150) if height//150<64 else 0
    cb=struct.pack('<I',2)+b'\x01'+b'\x00'*32+b'\xff'*4+compact(len(script))+script+b'\xff'*4+b'\x01'+struct.pack('<q',subsidy)+b'\x22\x51\x20'+owner+b'\x00'*4
    transaction=bytes.fromhex(txhex);merkle=sha256d(sha256d(cb)+sha256d(transaction))
    bits=int(header['bits'],16);target=(bits&0x7fffff)<<(8*((bits>>24)-3))
    prefix=struct.pack('<I',0x20000000)+bytes.fromhex(tip)[::-1]+merkle+struct.pack('<II',max(int(time.time()),header['mediantime']+1),bits)
    for nonce in range(1<<32):
        hdr=prefix+struct.pack('<I',nonce)
        if int.from_bytes(sha256d(hdr),'little')<=target:return (hdr+b'\x02'+cb+transaction).hex()
    raise RuntimeError('Could not solve test block')

def reject_both(state,label,tx,expected):
    rawblock=bad_block(state,tx['hex']);checks=[]
    # Prevent peer relay from reaching node 1 before its explicit submitblock call.
    # Each node must validate the attack independently rather than return a cached result.
    for n in state['nodes']:rpc(n,'setnetworkactive',False)
    for i,n in enumerate(state['nodes']):
        tip=rpc(n,'getbestblockhash');m=rpc(n,'testmempoolaccept',[tx['hex']])[0]
        assert not m.get('allowed',False),(label,m)
        b=rpc(n,'submitblock',rawblock)
        assert b is not None,(label,'invalid block accepted')
        mempool_expected=expected in str(m) or (expected=='missingorspent' and 'missing-inputs' in str(m))
        assert mempool_expected and expected in str(b),(label,m,b)
        assert rpc(n,'getbestblockhash')==tip
        checks.append({'node':i,'mempool':m,'block_rejection':b})
    connect(state);sync(state)
    return {'case':label,'checks':checks}

def status(state):
    result={'run':state['run'],'nodes':[]}
    for i,n in enumerate(state['nodes']):
        info=rpc(n,'getblockchaininfo');result['nodes'].append({'node':i,'rpc_port':n['rpc_port'],'p2p_port':n['p2p_port'],'height':info['blocks'],'tip':info['bestblockhash'],'peers':rpc(n,'getconnectioncount'),'mempool':rpc(n,'getrawmempool')})
    result['confirmed_confidential_balances_sats']=balances(recover_all(state))
    return result

def demo():
    if not DAEMON.exists() or not WALLET.exists():raise RuntimeError('Run integration/build.ps1 first')
    if LATEST.exists():
        old=load()
        for n in old['nodes']:stop_node(n)
    RUNS.mkdir(exist_ok=True)
    run=RUNS/time.strftime('%Y%m%d-%H%M%S');run.mkdir()
    ports=free_ports(4)
    state={'run':str(run),'nodes':[{'datadir':str(run/f'node{i}'),'rpc_port':ports[i*2],'p2p_port':ports[i*2+1]} for i in range(2)]}
    for name in ('alice','bob'):save(run/f'{name}.seed.json',{'seed':os.urandom(32).hex()})
    save(LATEST,state);start(state)
    report={'started':time.strftime('%Y-%m-%dT%H:%M:%S'),'scope':'isolated patched Bitcoin Core regtest; no real BTC','checks':[]}
    print('Mining 101 transparent blocks on node 0 and syncing node 1...',flush=True)
    mine(state,101);sync(state)
    genesis_fund=rpc(state['nodes'][0],'getblock',rpc(state['nodes'][0],'getblockhash',1),2)['tx'][0]
    funding=wallet('fund',seed=seed(state,'alice'),txid=genesis_fund['txid'],vout=0,value=50*SAT)
    deposit=wallet('build',inputs=[funding],payments=[{'address':identity(state,'alice')['address'],'value':10*SAT},{'owner':identity(state,'alice')['owner'],'value':40*SAT-1000}],fee=1000)
    broadcast_and_mine(state,deposit)
    report['conversion_txid']=deposit['txid'];print('Transparent -> confidential conversion mined by real Core nodes.',flush=True)
    pay=prepare_payment(state,'alice','bob',3*SAT);broadcast_and_mine(state,pay)
    owned=recover_all(state);bob_original=owned['bob'][0]
    spend=prepare_payment(state,'bob','alice',SAT);broadcast_and_mine(state,spend)
    before=recover_all(state)
    report['payment_txid']=pay['txid'];report['bob_spend_txid']=spend['txid']
    print('Bob received and spent a confidential payment. Deleting wallet caches...',flush=True)
    for name in ('alice','bob'):
        cache=run/f'{name}.wallet.json'
        assert cache.parent.resolve()==run.resolve() and cache.suffix=='.json'
        cache.unlink()
    restored=recover_all(state)
    assert restored==before
    report['seed_recovery']={'wallet_caches_deleted':True,'exact_coin_records_restored':True,'balances_sats':balances(restored)}
    bob_coin=restored['bob'][0]
    inflated=wallet('build',inputs=[bob_coin],payments=[{'address':identity(state,'alice')['address'],'value':bob_coin['value']+SAT}],fee=1000)
    report['checks'].append(reject_both(state,'inflation',inflated,'bad-cnu-proof-or-ownership'))
    legitimate=prepare_payment(state,'bob','alice',SAT//2)
    corrupted=wallet('tamper_proof',payload=legitimate['payload'])
    report['checks'].append(reject_both(state,'malformed_range_proof',corrupted,'bad-cnu-proof-or-ownership'))
    mirrored=bytearray.fromhex(legitimate['hex'])
    commitment=bytes.fromhex(legitimate['outputs'][0])[33:66]
    mirrored[mirrored.index(commitment)+5]^=1
    mismatch={'hex':mirrored.hex(),'txid':sha256d(mirrored)[::-1].hex()}
    report['checks'].append(reject_both(state,'native_output_payload_mismatch',mismatch,'bad-cnu-proof-or-ownership'))
    double=wallet('build',inputs=[bob_original],payments=[{'address':identity(state,'alice')['address'],'value':bob_original['value']-1000}],fee=1000)
    report['checks'].append(reject_both(state,'double_spend',double,'missingorspent'))
    print('Both nodes rejected inflation, malformed proofs and double spends in mempool AND solved blocks.',flush=True)
    print('Partitioning nodes and forcing a longer competing chain...',flush=True)
    a,b=state['nodes'];fork_parent=rpc(a,'getbestblockhash')
    for n in (a,b):rpc(n,'setnetworkactive',False)
    rpc(a,'sendrawtransaction',legitimate['hex'],0);branch_a=mine(state,1,0)[0]
    on_a=recover_all(state,0,write=False);assert balances(on_a)!=balances(restored)
    branch_b=mine(state,2,1);connect(state);sync(state)
    assert rpc(a,'getbestblockhash')==branch_b[-1]
    after_a=recover_all(state,0);after_b=recover_all(state,1,write=False)
    assert after_a==after_b==restored
    for n in (a,b):
        assert rpc(n,'gettxout',bob_coin['txid'],bob_coin['vout'],False) is not None
        assert rpc(n,'gettxout',legitimate['txid'],0,False) is None
    report['reorg']={'parent':fork_parent,'orphaned_block':branch_a,'winning_tip':branch_b[-1],'both_nodes_restored_input':True,'orphaned_outputs_absent':True,'both_wallet_replays_agree':True,'balances_after_reorg_sats':balances(after_a)}
    save(run/'checkpoint.json',{'phase':'after_reorg','report':report,'transaction':legitimate})
    finish_demo(state,report,legitimate)

def finish_demo(state,report,legitimate):
    a,b=state['nodes'];run=Path(state['run'])
    # Confirm the recovered wallet can spend, and leave no ambiguous pending demo payment.
    if legitimate['txid'] not in rpc(a,'getrawmempool'):
        rpc(a,'sendrawtransaction',legitimate['hex'],0)
    if legitimate['txid'] not in rpc(b,'getrawmempool'):
        assert rpc(b,'testmempoolaccept',[legitimate['hex']])[0].get('allowed')
        rpc(b,'sendrawtransaction',legitimate['hex'],0)
    report['reorg']['explicit_wallet_resubmission']=True
    mine(state,1,1);sync(state);final_wallets=recover_all(state)
    print('Restarting node 0 and rebuilding node 1 chainstate from blocks...',flush=True)
    final_tip=rpc(a,'getbestblockhash');final_height=rpc(a,'getblockcount')
    stop_node(a);start_node(a)
    stop_node(b);start_node(b,['-reindex-chainstate=1'])
    wait(lambda:rpc(b,'getblockcount')==final_height,'chainstate rebuild',180)
    connect(state);sync(state)
    assert rpc(a,'getbestblockhash')==rpc(b,'getbestblockhash')==final_tip
    assert recover_all(state,1,write=False)==final_wallets
    for n in (a,b):assert rpc(n,'verifychain',4,0)
    report['persistence']={'restart_verified':True,'node1_reindexed_from_blocks':True,'verifychain_both_nodes':True}
    report['final']=status(state)
    report['binary_sha256']=hashlib.sha256(DAEMON.read_bytes()).hexdigest()
    report['completed']=time.strftime('%Y-%m-%dT%H:%M:%S');report['success']=True
    save(run/'report.json',report);save(LAB/'report.json',report);save(LATEST,state)
    print(json.dumps({'success':True,'report':str(LAB/'report.json'),'nodes_left_running':True,'balances_sats':balances(final_wallets)},indent=2))

def resume_after_reorg(state):
    """Resume the already-mined lab without recreating wallets, nodes or its chain."""
    run=Path(state['run']);checkpoint=run/'checkpoint.json'
    if checkpoint.exists():
        c=json.loads(checkpoint.read_text())
        if c['phase']!='after_reorg':raise RuntimeError('Unsupported checkpoint phase')
        finish_demo(state,c['report'],c['transaction']);return
    # Recover the pre-checkpoint harness's exact state using persisted Core evidence.
    a,b=state['nodes'];sync(state)
    pending=rpc(a,'getrawmempool');assert len(pending)==1,pending
    tx=rpc(a,'getrawtransaction',pending[0],True)
    tips=rpc(a,'getchaintips');forks=[t for t in tips if t['status']=='valid-fork']
    orphan=next(t for t in forks if pending[0] in [x['txid'] for x in rpc(a,'getblock',t['hash'],2)['tx']])
    orphan_block=rpc(a,'getblock',orphan['hash'],2)
    restored=recover_all(state);assert restored==recover_all(state,1,write=False)
    for n in (a,b):
        for i in tx['vin']:assert rpc(n,'gettxout',i['txid'],i['vout'],False) is not None
        for o in tx['vout']:assert rpc(n,'gettxout',tx['txid'],o['n'],False) is None
    checks=[];pattern=r'ConnectBlock ([0-9a-f]{64}) failed, (bad-cnu-proof-or-ownership|bad-txns-inputs-missingorspent),.*?in transaction ([0-9a-f]{64})'
    logs=[(Path(n['datadir'])/'regtest/debug.log').read_text(errors='replace') for n in (a,b)]
    events=list(dict.fromkeys(re.findall(pattern,logs[0])))
    assert len(events)==4,events
    for label,(block,reason,tid) in zip(['inflation','malformed_range_proof','native_output_payload_mismatch','double_spend'],events):
        assert (block,reason,tid) in re.findall(pattern,logs[1])
        checks.append({'case':label,'block':block,'transaction':tid,'both_nodes_connectblock_rejection':reason,'evidence':'persisted independent node debug.log; earlier mempool assertions passed but were not checkpointed'})
    transactions={}
    for height,label in [(102,'conversion_txid'),(103,'payment_txid'),(104,'bob_spend_txid')]:
        mined=rpc(a,'getblock',rpc(a,'getblockhash',height),2)
        transactions[label]=next(t['txid'] for t in mined['tx'] if 'cnu_payload' in t)
    report={'scope':'isolated patched Bitcoin Core regtest; no real BTC','resumed_existing_run':True,**transactions,'checks':checks,
        'seed_recovery':{'wallet_caches_deleted':True,'exact_coin_records_restored':True,'balances_sats':balances(restored),'evidence':'passed before interruption; current seed-and-chain rescan also agrees on both nodes'},
        'reorg':{'parent':orphan_block['previousblockhash'],'orphaned_block':orphan['hash'],'winning_tip':rpc(a,'getbestblockhash'),'both_nodes_restored_input':True,'orphaned_outputs_absent':True,'both_wallet_replays_agree':True,'balances_after_reorg_sats':balances(restored)}}
    c={'phase':'after_reorg','report':report,'transaction':{'txid':tx['txid'],'hex':tx['hex'],'payload':tx['cnu_payload']}}
    save(checkpoint,c);finish_demo(state,report,c['transaction'])

def main():
    command=sys.argv[1] if len(sys.argv)>1 else 'status'
    if command=='demo':demo();return
    s=load()
    if command=='start':start(s);print(json.dumps(status(s),indent=2))
    elif command=='resume':resume_after_reorg(s)
    elif command=='stop':
        for n in s['nodes']:stop_node(n)
        print('Both lab nodes stopped. Chain data and test seeds preserved.')
    elif command=='status':print(json.dumps(status(s),indent=2))
    elif command=='recover':
        name=sys.argv[2] if len(sys.argv)>2 else 'bob'
        if name not in ('alice','bob'):raise ValueError('Choose alice or bob')
        p=Path(s['run'])/f'{name}.wallet.json'
        if p.exists():p.unlink()
        print(json.dumps({'recovered_from_seed_and_blocks':name,'balances_sats':balances(recover_all(s))},indent=2))
    elif command=='pay':
        sender,receiver,value=sys.argv[2:5]
        if sender not in ('alice','bob') or receiver not in ('alice','bob'):raise ValueError('Choose alice or bob')
        tx=prepare_payment(s,sender,receiver,int(value));block=broadcast_and_mine(s,tx)
        print(json.dumps({'txid':tx['txid'],'block':block,'balances_sats':balances(recover_all(s))},indent=2))
    else:raise ValueError('Expected demo, start, stop, status, recover or pay')

if __name__=='__main__':main()
