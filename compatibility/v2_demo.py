"""V2: fragmented backing, reserve-free transfers, bound native txids, experimental relay."""
from __future__ import annotations
import copy
import hashlib
import json
import struct
import sys
import time
from decimal import Decimal
from pathlib import Path
import demo as v1
from demo import lab, CTransaction, CTxIn, CTxOut, CTxInWitness, CScript, outpoint, txid, native_output, create_coinbase, create_block, add_witness_commitment, sign_schnorr, TaprootSignatureHash
from test_framework.p2p import NetworkThread, P2PInterface
from test_framework.messages import msg_tx, msg_inv, CInv, MSG_TX

HERE=Path(__file__).resolve().parent
RESERVE=b'\x5f\x20'+b'\x44'*32
MAGIC=b'\x50CNU2'
SAT=100_000_000
FEE=1000

def envelope(payload,outputs):
    p=bytes.fromhex(payload)
    return MAGIC+struct.pack('<I',len(p))+p+struct.pack('<I',len(outputs))+b''.join(bytes.fromhex(o) for o in outputs)

def decode_envelope(b):
    assert b[:5]==MAGIC
    n=int.from_bytes(b[5:9],'little');payload=b[9:9+n];at=9+n
    count=int.from_bytes(b[at:at+4],'little');at+=4;outputs=[]
    for _ in range(count):
        size=41 if b[at]==0 else 122
        outputs.append(b[at:at+size].hex());at+=size
    assert at==len(b)
    return payload.hex(),outputs

def outer_hash(tx):
    tag=hashlib.sha256(b'CNU/outer/v2').digest()
    return hashlib.sha256(tag+tag+tx.serialize_without_witness()).digest()

def select_reserves(available,required,excluded=()):
    """Smallest adequate single fragment, otherwise descending values; no global lock."""
    blocked=set(excluded)
    coins=[r for r in available if (r['txid'],r['vout']) not in blocked]
    if required<=0:return []
    enough=[r for r in coins if r['value']>=required]
    if enough:return [min(enough,key=lambda r:(r['value'],r['txid'],r['vout']))]
    selected=[];total=0
    for r in sorted(coins,key=lambda r:(-r['value'],r['txid'],r['vout'])):
        selected.append(r);total+=r['value']
        if total>=required:return selected
    raise ValueError('Insufficient available backing; refresh exclusions after conflicts.')

class Demo(v1.Demo):
    def __init__(self):
        super().__init__()
        ports=lab.free_ports(2)
        self.nodes.append({'datadir':str(self.run/'upgraded-peer'),'rpc_port':ports[0],'p2p_port':ports[1],'flavor':'peer'})
        self.report['scope']='V2 stock consensus compatibility plus two upgraded experimental relay nodes'
        self.report['version']=2
        self.reserves={}
        lab.save(HERE/'latest-v2.json',{'run':str(self.run),'nodes':self.nodes})

    def save(self):
        lab.save(self.run/'report.json',self.report);lab.save(HERE/'report-v2.json',self.report)

    def binary(self,n):return HERE/f'bin/{"stock" if n["flavor"]=="stock" else "upgraded"}/bitcoind.exe'

    def start(self):
        for n in self.nodes:
            lab.DAEMON=self.binary(n);lab.start_node(n,extra=('-acceptnonstdtxn=0',))
        self.report['binaries']={n['flavor']:hashlib.sha256(self.binary(n).read_bytes()).hexdigest() for n in self.nodes}
        lab.rpc(self.nodes[1],'addnode',f'127.0.0.1:{self.nodes[2]["p2p_port"]}','onetry')
        lab.wait(lambda:lab.rpc(self.nodes[1],'getconnectioncount')==1,'upgraded peers')
        self.save()

    def block(self,txs=(),parent=None):
        parent=parent or lab.rpc(self.nodes[0],'getbestblockhash')
        hdr=lab.rpc(self.nodes[0],'getblockheader',parent);height=hdr['height']+1
        name='bob' if height==2 else 'alice'
        cb=create_coinbase(height,script_pubkey=CScript(b'\x51\x20'+bytes.fromhex(self.identities[name]['owner'])))
        b=create_block(int(parent,16),cb,ntime=max(int(time.time()),hdr['time']+1),txlist=list(txs),version=0x20000000)
        add_witness_commitment(b);b.solve();return b

    def submit(self,b,side_branch=False):
        for n in self.nodes:
            result=lab.rpc(n,'submitblock',b.serialize().hex())
            assert result in (None,'duplicate') or (side_branch and result=='inconclusive'),(n['flavor'],result)
        if not side_branch:
            for n in self.nodes:assert lab.rpc(n,'getbestblockhash')==b.hash_hex

    def build(self,inputs,payments,fee=FEE,reserves=(),split=None):
        made=lab.wallet('build',inputs=inputs,payments=payments,fee=fee)
        outputs=[native_output(o) for o in made['outputs']]
        prior=[CTxOut(r['value'],CScript(RESERVE)) for r in reserves]+[native_output(c['output']) for c in inputs]
        backing=sum(o.nValue for o in prior)-sum(o.nValue for o in outputs)-fee
        assert backing>=0
        amounts=split if split is not None else ([10*SAT]*(backing//(10*SAT))+([backing%(10*SAT)] if backing%(10*SAT) else []))
        assert sum(amounts)==backing and all(v>0 for v in amounts)
        tx=CTransaction()
        tx.vin=[CTxIn(outpoint(c['txid'],c['vout'])) for c in [*reserves,*inputs]]
        for i in tx.vin:i.nSequence=0xffffffff
        tx.vout=[CTxOut(v,CScript(RESERVE)) for v in amounts]+outputs
        tx.wit.vtxinwit=[CTxInWitness() for _ in tx.vin]
        env=envelope(made['payload'],made['outputs']);h=outer_hash(tx)
        for j,c in enumerate(inputs):
            i=len(reserves)+j;meta=bytes.fromhex(c['output']);first=j==0
            if meta[0]==1:
                stack=[meta,sign_schnorr(bytes.fromhex(c['secret']),h)]
                if first:stack.append(env)
            else:
                sig=sign_schnorr(bytes.fromhex(c['secret']),TaprootSignatureHash(tx,prior,0,input_index=i,annex=env if first else None))
                stack=[sig]+([env] if first else [])
            tx.wit.vtxinwit[i].scriptWitness.stack=stack
        made.update(offset=len(amounts),reserve_inputs=len(reserves),inputs=inputs,reserves=list(reserves))
        return tx,made

    def record(self,tx,made,label):
        offset=made['offset'];spent={(i.prevout.hash,i.prevout.n) for i in tx.vin}
        for op in list(self.reserves):
            if (int(op[0],16),op[1]) in spent:self.reserves.pop(op)
        for i in range(offset):self.reserves[(txid(tx),i)]={'txid':txid(tx),'vout':i,'value':tx.vout[i].nValue}
        for name in self.coins:
            self.coins[name]=[c for c in self.coins[name] if (int(c['txid'],16),c['vout']) not in spent]
            coins=lab.wallet('scan',seed=self.seeds[name],payload=made['payload'],previous=[c['output'] for c in made['inputs']],native_txid=txid(tx),output_offset=offset)['coins']
            self.coins[name].extend(coins)
        item={'case':label,'txid':txid(tx),'reserve_inputs':made['reserve_inputs'],'reserve_outputs':offset,'bytes':len(tx.serialize()),'vbytes':tx.get_vsize()}
        self.report['transactions'].append(item);print(json.dumps(item),flush=True);self.save()

    def transparent_coin(self,tx,index,name):
        return lab.wallet('fund',seed=self.seeds[name],txid=txid(tx),vout=index,value=tx.vout[index].nValue)

    def relay(self,tx):
        accepted=lab.rpc(self.nodes[1],'sendrawtransaction',tx.serialize().hex())
        assert accepted==txid(tx)
        lab.wait(lambda:txid(tx) in lab.rpc(self.nodes[2],'getrawmempool'),'peer transaction relay')

    def submit_with_reselection(self,inputs,payments,reserves,fee=FEE,max_attempts=4):
        """Retry only observed backing conflicts; preserve signed payment intent.

        gettxout is a local snapshot, not a reservation. The bounded retry also
        handles races after selection. Claim-input conflicts are never retried.
        """
        inputs=copy.deepcopy(inputs);payments=copy.deepcopy(payments)
        attempts=[]
        for attempt in range(max_attempts):
            tx,made=self.build(inputs,payments,fee=fee,reserves=reserves)
            try:
                self.relay(tx)
                return tx,made,attempts
            except lab.RPCError:
                node=self.nodes[1]
                if any(lab.rpc(node,'gettxout',c['txid'],c['vout'],True) is None for c in inputs):raise
                if not any(lab.rpc(node,'gettxout',r['txid'],r['vout'],True) is None for r in reserves):raise
                attempts.append(txid(tx))
                available=[r for r in self.reserves.values() if lab.rpc(node,'gettxout',r['txid'],r['vout'],True) is not None]
                required=sum(p.get('value',0) for p in payments if 'owner' in p)+fee-sum(native_output(c['output']).nValue for c in inputs)
                reserves=select_reserves(available,required)
        raise RuntimeError('Backing contention persisted; payment remains unsubmitted after bounded retries')

    def witness_recovery(self,tx):
        network=NetworkThread();network.start();peer=None
        try:
            lab.wait(lambda:NetworkThread.network_event_loop is not None,'test P2P loop')
            peer=P2PInterface(wtxidrelay=False)
            peer.peer_connect(dstaddr='127.0.0.1',dstport=self.nodes[1]['p2p_port'],net='regtest',timeout_factor=1,supports_v2_p2p=False,send_version=True)()
            peer.wait_until(lambda:peer.is_connected,check_connected=False,timeout=10)
            peer.wait_for_verack(timeout=10)
            stripped=CTransaction(tx);stripped.wit.vtxinwit=[]
            assert txid(stripped)==txid(tx)
            peer.send_and_ping(msg_tx(stripped),timeout=10)
            assert txid(tx) not in lab.rpc(self.nodes[1],'getrawmempool')
            # A legacy txid announcement must still trigger a download after witness stripping.
            peer.send_without_ping(msg_inv([CInv(MSG_TX,int(txid(tx),16))]))
            peer.wait_until(lambda:'getdata' in peer.last_message and any(i.hash==int(txid(tx),16) for i in peer.last_message['getdata'].inv),timeout=15)
            peer.send_and_ping(msg_tx(tx),timeout=10)
            lab.wait(lambda:txid(tx) in lab.rpc(self.nodes[1],'getrawmempool'),'valid witness after stripped copy')
            lab.wait(lambda:txid(tx) in lab.rpc(self.nodes[2],'getrawmempool'),'recovered witness relay')
            self.report['witness_recovery']={'stripped_copy_same_txid':True,'legacy_inv_requested_valid_copy':True,'valid_copy_accepted_and_relayed':True}
        finally:
            if peer is not None:
                peer.peer_disconnect()
                peer.wait_until(lambda:not peer.is_connected,check_connected=False,timeout=10)
            network.close(timeout=10)

    def attack(self,label,tx,expected='bad-cnu-reserve',stock_accepts=True):
        # No invalid block is announced to the upgraded peer by the stock node.
        parent=lab.rpc(self.nodes[0],'getbestblockhash');b=self.block([tx]);raw=b.serialize().hex()
        stock=lab.rpc(self.nodes[0],'submitblock',raw)
        results=[lab.rpc(n,'submitblock',raw) for n in self.nodes[1:]]
        if stock_accepts:assert stock is None,(label,stock)
        else:assert stock==expected,(label,stock)
        # The second upgraded peer may already know the failed block via its peer.
        assert results[0]==expected,(label,results)
        assert results[1] in (expected,'duplicate-invalid'),(label,results)
        for n in self.nodes[1:]:assert lab.rpc(n,'getbestblockhash')==parent
        if stock_accepts:lab.rpc(self.nodes[0],'invalidateblock',b.hash_hex)
        item={'case':label,'stock_result':stock,'upgraded_results':results,'block':b.hash_hex}
        self.report['checks'].append(item);print(json.dumps(item),flush=True);self.save()

    def execute(self):
        self.start();funds={}
        for h in range(1,104):
            b=self.block();self.submit(b)
            if h in (1,2):
                name='alice' if h==1 else 'bob';funds[name]=self.transparent_coin(b.vtx[0],0,name)
        deposits={};sponsors={}
        for name in ('alice','bob'):
            payments=[{'address':self.identities[name]['address'],'value':10*SAT}]
            payments += [{'owner':self.identities[name]['owner'],'value':20000} for _ in range(4)]
            payments += [{'owner':self.identities[name]['owner'],'value':40*SAT-80000-FEE}]
            tx,made=self.build([funds[name]],payments,split=[4*SAT,6*SAT]);deposits[name]=(tx,made)
            sponsors[name]=[self.transparent_coin(tx,made['offset']+1+i,name) for i in range(4)]
            self.relay(tx)
        assert len(lab.rpc(self.nodes[1],'getrawmempool'))==2
        self.submit(self.block([v[0] for v in deposits.values()]))
        for name,(tx,made) in deposits.items():self.record(tx,made,f'independent_{name}_deposit')
        self.report['independent_deposits']={'coexisted_in_mempool':True,'reserve_inputs':0,'same_block':True,'coinbase_bootstrap':False}
        a,am=self.build(self.coins['alice']+[sponsors['alice'][0]],[{'address':self.identities['bob']['address'],'value':SAT},{'address':self.identities['alice']['address'],'value':9*SAT},{'owner':self.identities['alice']['owner'],'value':19000}])
        b,bm=self.build(self.coins['bob']+[sponsors['bob'][0]],[{'address':self.identities['alice']['address'],'value':2*SAT},{'address':self.identities['bob']['address'],'value':4*SAT},{'address':self.identities['bob']['address'],'value':4*SAT},{'owner':self.identities['bob']['owner'],'value':19000}])
        child_coin=lab.wallet('scan',seed=self.seeds['bob'],payload=am['payload'],previous=[c['output'] for c in am['inputs']],native_txid=txid(a),output_offset=0)['coins'][0]
        child,cm=self.build([child_coin,sponsors['bob'][1]],[{'address':self.identities['alice']['address'],'value':SAT},{'owner':self.identities['bob']['owner'],'value':19000}])
        assert not ({i.prevout.serialize() for i in a.vin}&{i.prevout.serialize() for i in b.vin})
        self.witness_recovery(a)
        for tx in (b,child):self.relay(tx)
        assert len(lab.rpc(self.nodes[1],'getrawmempool'))==3
        self.submit(self.block([a,b,child]))
        for tx,made,label in ((a,am,'alice_payment'),(b,bm,'bob_payment'),(child,cm,'unconfirmed_child')):self.record(tx,made,label)
        self.report['concurrency']={'independent_transfers_and_child_in_mempool':3,'reserve_inputs':0,'peer_relay':True,'require_standard':True,'unchanged_parent_child_txids':True}

        alice=[c for c in self.coins['alice'] if c['value']==9*SAT]
        bob=[self.coins['bob'][0]]
        ra=self.reserves[(txid(deposits['alice'][0]),0)];rb=self.reserves[(txid(deposits['bob'][0]),0)]
        wa,wam=self.build(alice,[{'address':self.identities['alice']['address'],'value':8*SAT-FEE},{'owner':self.identities['alice']['owner'],'value':SAT}],reserves=[ra])
        wb,wbm=self.build(bob,[{'address':self.identities['bob']['address'],'value':2*SAT-FEE},{'owner':self.identities['bob']['owner'],'value':2*SAT}],reserves=[rb])
        wrong=CTransaction(wa);wrong.vin[0].prevout=outpoint(txid(deposits['alice'][0]),1);wrong.vout[0].nValue+=2*SAT
        self.attack('third_party_reserve_rebase',wrong)
        extra,_=self.build(alice,[{'address':self.identities['alice']['address'],'value':8*SAT-FEE},{'owner':self.identities['alice']['owner'],'value':SAT}],reserves=[ra,self.reserves[(txid(deposits['alice'][0]),1)]])
        self.attack('unnecessary_reserve_churn',extra)
        block3=lab.rpc(self.nodes[0],'getblock',lab.rpc(self.nodes[0],'getblockhash',3),2)
        fresh=lab.wallet('fund',seed=self.seeds['alice'],txid=block3['tx'][0]['txid'],vout=0,value=50*SAT)
        oversized,_=self.build([fresh],[{'address':self.identities['alice']['address'],'value':12*SAT},{'owner':self.identities['alice']['owner'],'value':38*SAT-FEE}],split=[12*SAT])
        self.attack('oversized_reserve_fragment',oversized)
        normal=CTransaction();normal.vin=[CTxIn(outpoint(fresh['txid'],fresh['vout']))];normal.vin[0].nSequence=0xffffffff
        normal.vout=[CTxOut(50*SAT-FEE,CScript(b'\x51\x20'+bytes.fromhex(self.identities['alice']['owner'])))]
        normal.wit.vtxinwit=[CTxInWitness()];annex=MAGIC+b'unrelated-annex'
        normal.wit.vtxinwit[0].scriptWitness.stack=[sign_schnorr(bytes.fromhex(fresh['secret']),TaprootSignatureHash(normal,[native_output(fresh['output'])],0,annex=annex)),annex]
        controls=[lab.rpc(n,'testmempoolaccept',[normal.serialize().hex()])[0] for n in self.nodes]
        assert all(not c['allowed'] and c['reject-reason']=='bad-witness-nonstandard' for c in controls),controls
        self.report['unrelated_annex_policy_preserved']=True
        # Complete cryptographically valid inner proofs cannot hide an extra BTC.
        fraud,fm=self.build([self.coins['bob'][1],sponsors['bob'][2]],[{'address':self.identities['bob']['address'],'value':5*SAT},{'owner':self.identities['bob']['owner'],'value':19000}])
        self.attack('hidden_inflation',fraud)
        tampered=CTransaction(wa);env=bytearray(tampered.wit.vtxinwit[1].scriptWitness.stack[-1]);env[9+50]^=1;tampered.wit.vtxinwit[1].scriptWitness.stack[-1]=bytes(env)
        self.attack('tampered_payload',tampered)
        bad=CTransaction(wa);corrupt=lab.wallet('tamper_proof',payload=wam['payload'])
        bad.wit.vtxinwit[1].scriptWitness.stack[-1]=envelope(corrupt['payload'],wam['outputs'])
        self.attack('malformed_range_proof',bad)
        bad=CTransaction(wa);bad.wit.vtxinwit[1].scriptWitness.stack[1]=bytes(64);self.attack('forged_outer_signature',bad)
        missing=CTransaction(wa);missing.wit.vtxinwit[1].scriptWitness.stack.pop();self.attack('missing_proof_data',missing)
        forged=CTransaction(wa);meta=bytearray(forged.wit.vtxinwit[1].scriptWitness.stack[0]);meta[-1]^=1;forged.wit.vtxinwit[1].scriptWitness.stack[0]=bytes(meta)
        self.attack('forged_previous_coin_metadata',forged)
        theft=CTransaction();theft.vin=[CTxIn(outpoint(alice[0]['txid'],alice[0]['vout']))];theft.vout=[CTxOut(0,CScript(b'\x51'))]
        self.attack('ordinary_carrier_theft',theft)
        theft=CTransaction();theft.vin=[CTxIn(outpoint(ra['txid'],ra['vout']))];theft.vout=[CTxOut(ra['value']-FEE,CScript(b'\x51'))]
        self.attack('unauthorized_reserve_withdrawal',theft)

        self.relay(wa)
        exit_payments=[{'address':self.identities['bob']['address'],'value':2*SAT-FEE},{'owner':self.identities['bob']['owner'],'value':2*SAT}]
        conflict,fcm=self.build(bob,exit_payments,reserves=[ra])
        old_coin=lab.wallet('scan',seed=self.seeds['bob'],payload=fcm['payload'],previous=[c['output'] for c in bob],native_txid=txid(conflict),output_offset=fcm['offset'])['coins'][0]
        child_payments=[{'address':self.identities['bob']['address'],'value':old_coin['value']},{'owner':self.identities['bob']['owner'],'value':19000}]
        stale_child,_=self.build([old_coin,sponsors['bob'][2]],child_payments)
        wb,wbm,retries=self.submit_with_reselection(bob,exit_payments,[ra])
        assert len(retries)==1 and txid(wb)!=txid(conflict)
        assert not lab.rpc(self.nodes[1],'testmempoolaccept',[stale_child.serialize().hex()])[0]['allowed']
        new_coin=lab.wallet('scan',seed=self.seeds['bob'],payload=wbm['payload'],previous=[c['output'] for c in bob],native_txid=txid(wb),output_offset=wbm['offset'])['coins'][0]
        rebuilt_child,_=self.build([new_coin,sponsors['bob'][2]],child_payments)
        assert lab.rpc(self.nodes[1],'testmempoolaccept',[rebuilt_child.serialize().hex()])[0]['allowed']
        self.report['withdrawal_retry']={'failed_backing_attempts':len(retries),'reselected_and_relayed':True,'amounts_preserved':new_coin['value']==old_coin['value'],'stale_descendant_rejected':True,'rebuilt_descendant_accepted':True,'retry_limit':4}
        self.report['independent_withdrawals']={'disjoint_fragments':True,'coexisted_in_mempool':True}
        pending,pm=self.build([self.coins['bob'][1],sponsors['bob'][3]],[{'address':self.identities['alice']['address'],'value':SAT//2},{'address':self.identities['bob']['address'],'value':4*SAT-SAT//2},{'owner':self.identities['bob']['owner'],'value':19000}])
        pending_bytes=pending.serialize()
        snapshot=(copy.deepcopy(self.coins),copy.deepcopy(self.reserves))
        parent=lab.rpc(self.nodes[0],'getbestblockhash')
        branch_a=self.block([wa,wb]);self.submit(branch_a)
        alternative,altm=self.build(alice,[{'address':self.identities['alice']['address'],'value':9*SAT-SAT//2-FEE},{'owner':self.identities['alice']['owner'],'value':SAT//2}],reserves=[ra])
        branch_b=self.block([alternative],parent=parent);self.submit(branch_b,True)
        winner=self.block(parent=branch_b.hash_hex);self.submit(winner)
        self.coins,self.reserves=snapshot;self.record(alternative,altm,'competing_branch_withdrawal')
        # A separately prepared transfer and withdrawal remain byte-for-byte usable.
        assert pending.serialize()==pending_bytes
        self.submit(self.block([wb,pending]));self.record(wb,wbm,'unaffected_withdrawal_after_reorg');self.record(pending,pm,'reserve_free_transfer_after_reorg')
        self.report['reorg']={'orphaned':branch_a.hash_hex,'winning':winner.hash_hex,'competing_reserve_spend':True,'unaffected_withdrawal_txid_stable':True,'prebuilt_transfer_unchanged':True}
        chosen=select_reserves(list(self.reserves.values()),7*SAT+FEE)
        assert len(chosen)==2
        alice_value=sum(c['value'] for c in self.coins['alice'])
        merged,mm=self.build(self.coins['alice'].copy(),[{'address':self.identities['alice']['address'],'value':alice_value-7*SAT-FEE},{'owner':self.identities['alice']['owner'],'value':7*SAT}],reserves=chosen)
        self.relay(merged);self.submit(self.block([merged]));self.record(merged,mm,'multi_fragment_withdrawal')
        self.report['fragment_selection']={'multiple_inputs_supported':True,'inputs':len(chosen),'change_fragments':mm['offset']}
        duplicate=CTransaction(pending);duplicate.nLockTime=1;self.attack('double_spend',duplicate,'bad-txns-inputs-missingorspent',False)
        self.report['validation_cost_samples']=[]
        value=sum(c['value'] for c in self.coins['alice'])
        for count in (2,8,24):
            each=value//count
            amounts=[each]*(count-1)+[value-each*(count-1)]
            inputs=self.coins['alice']+[sponsors['alice'][3]]
            payments=[{'address':self.identities['alice']['address'],'value':v} for v in amounts]+[{'owner':self.identities['alice']['owner'],'value':19000}]
            large,lm=self.build(inputs,payments)
            begin=time.perf_counter();result=lab.rpc(self.nodes[1],'testmempoolaccept',[large.serialize().hex()])[0];valid_ms=(time.perf_counter()-begin)*1000
            assert result['allowed'],result
            begin=time.perf_counter();warm=lab.rpc(self.nodes[1],'testmempoolaccept',[large.serialize().hex()])[0];warm_ms=(time.perf_counter()-begin)*1000
            assert warm['allowed'],warm
            # Corrupt the last range proof so the verifier must process preceding proofs.
            p=bytearray(bytes.fromhex(lm['payload']))
            at=12+36*len(inputs)+4+sum(len(bytes.fromhex(o)) for o in lm['outputs'])+12
            sigs=int.from_bytes(p[at:at+4],'little');at+=4+64*sigs
            ranges=int.from_bytes(p[at:at+4],'little');at+=4;last=None
            for _ in range(ranges):
                tag=p[at];at+=1
                if tag:
                    at+=33;n=int.from_bytes(p[at:at+4],'little');at+=4;last=at;at+=n
                    n=int.from_bytes(p[at:at+4],'little');at+=4+n
            assert last is not None;p[last+30]^=1
            invalid=CTransaction(large);invalid.wit.vtxinwit[0].scriptWitness.stack[-1]=envelope(p.hex(),lm['outputs'])
            begin=time.perf_counter();cold_invalid=lab.rpc(self.nodes[2],'testmempoolaccept',[invalid.serialize().hex()])[0];cold_invalid_ms=(time.perf_counter()-begin)*1000
            assert not cold_invalid['allowed'] and cold_invalid['reject-reason']=='bad-cnu-reserve',cold_invalid
            begin=time.perf_counter();result=lab.rpc(self.nodes[1],'testmempoolaccept',[invalid.serialize().hex()])[0];invalid_ms=(time.perf_counter()-begin)*1000
            assert not result['allowed'] and result['reject-reason']=='bad-cnu-reserve',result
            sample={'confidential_outputs':count,'bytes':len(large.serialize()),'vbytes':large.get_vsize(),'valid_testmempoolaccept_ms':round(valid_ms,2),'cold_peer_last_invalid_range_ms':round(cold_invalid_ms,2),'warm_valid_ms':round(warm_ms,2),'warm_prefix_last_invalid_range_ms':round(invalid_ms,2),'samples_per_case':1}
            self.report['validation_cost_samples'].append(sample);print(json.dumps(sample),flush=True);self.save()
        # Both users pay fees from hidden balances, with no transparent funding input.
        fee_txs=[];used=[]
        for name,recipient in (('alice','bob'),('bob','alice')):
            inputs=copy.deepcopy(self.coins[name]);value=sum(c['value'] for c in inputs)
            reserve=select_reserves(list(self.reserves.values()),FEE,used)
            used.extend((r['txid'],r['vout']) for r in reserve)
            tx,made=self.build(inputs,[{'address':self.identities[recipient]['address'],'value':10000},{'address':self.identities[name]['address'],'value':value-10000-FEE}],reserves=reserve)
            assert all(bytes.fromhex(c['output'])[0]==1 for c in inputs)
            assert all(o.nValue==0 for o in tx.vout[made['offset']:])
            self.relay(tx);fee_txs.append((tx,made,name))
        assert len(lab.rpc(self.nodes[1],'getrawmempool'))==2
        self.submit(self.block([t for t,_,_ in fee_txs]))
        for tx,made,name in fee_txs:self.record(tx,made,name+'_confidential_fee_payment')
        self.report['confidential_fee_mode']={'transparent_fee_inputs':0,'concurrent_disjoint_backing_payments':2,'fees_sats_each':FEE,'public_fee_and_backing_graph_remain':True}
        self.report['persistence']={}
        for n in self.nodes:lab.stop_node(n)
        for n in self.nodes:
            lab.DAEMON=self.binary(n);lab.start_node(n,extra=('-reindex-chainstate=1','-acceptnonstdtxn=0'))
            assert lab.rpc(n,'verifychain',4,0)
            for r in self.reserves.values():assert lab.rpc(n,'gettxout',r['txid'],r['vout'],False) is not None
            self.report['persistence'][n['flavor']]={'reindexed':True,'verifychain':True}
        for name,coins in self.coins.items():lab.save(self.run/f'{name}.cache.json',coins)
        for name in self.coins:(self.run/f'{name}.cache.json').unlink()
        for n in self.nodes:
            found=self.recover(n)
            for name in self.coins:
                key=lambda cs:{(c['txid'],c['vout']):c for c in cs}
                assert key(found[name])==key(self.coins[name])
                lab.save(self.run/f'{name}.cache.json',found[name])
        total=sum(r['value'] for r in self.reserves.values());assert total==sum(lab.balances(self.coins).values())
        self.report['seed_recovery']={'exact_records_all_three_nodes':True,'caches_deleted':True}
        self.report['final']={'height':lab.rpc(self.nodes[0],'getblockcount'),'tip':lab.rpc(self.nodes[0],'getbestblockhash'),'reserve_fragments':len(self.reserves),'backing_sats':total,'confidential_balances_sats':lab.balances(self.coins)}
        self.report['success']=True;self.save();print(json.dumps(self.report['final'],indent=2),flush=True)

    def recover(self,node):
        history={};found={n:{} for n in self.seeds}
        for h in range(1,lab.rpc(node,'getblockcount')+1):
            for tx in lab.rpc(node,'getblock',lab.rpc(node,'getblockhash',h),2)['tx']:
                envs=[bytes.fromhex(i.get('txinwitness',[''])[-1]) for i in tx['vin'] if i.get('txinwitness')]
                env=next((e for e in envs if e.startswith(MAGIC)),None);outputs=None;offset=0
                if env is not None and h>=getattr(self,'activation_height',0):
                    payload,outputs=decode_envelope(env)
                    while tx['vout'][offset]['scriptPubKey']['hex']==RESERVE.hex():offset+=1
                    prev=[history[(i['txid'],i['vout'])] for i in tx['vin'] if (i['txid'],i['vout']) in history]
                    for name in found:
                        for c in lab.wallet('scan',seed=self.seeds[name],payload=payload,previous=prev,native_txid=tx['txid'],output_offset=offset)['coins']:found[name][(c['txid'],c['vout'])]=c
                for i in tx['vin']:
                    if 'txid' in i:
                        op=(i['txid'],i['vout']);history.pop(op,None)
                        for w in found.values():w.pop(op,None)
                for o in tx['vout']:
                    script=bytes.fromhex(o['scriptPubKey']['hex']);op=(tx['txid'],o['n'])
                    if outputs is not None and o['n']>=offset:history[op]=outputs[o['n']-offset]
                    elif script[:2]==b'\x51\x20' and len(script)==34:history[op]=(b'\x00'+struct.pack('<Q',int(Decimal(str(o['value']))*SAT))+script[2:]).hex()
        return {name:list(w.values()) for name,w in found.items()}

if __name__=='__main__':
    demo=Demo()
    try:demo.execute()
    except Exception as e:demo.report['failure']=str(e);demo.save();raise
    finally:
        for node in demo.nodes:lab.stop_node(node)
