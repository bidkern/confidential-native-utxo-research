"""Differential block-consensus experiment: stock Core vs reserve-restricted Core.

Private regtest, isolated data directories, no public-network publication.
"""
from __future__ import annotations
import copy
import hashlib
import json
import secrets
import struct
import sys
import time
from decimal import Decimal
from datetime import datetime
from pathlib import Path

HERE=Path(__file__).resolve().parent
ROOT=HERE.parent
sys.path.insert(0,str(ROOT))
sys.path.insert(0,str(HERE/'bitcoin-core/test/functional'))
from integration import lab
from test_framework.blocktools import create_block, create_coinbase, add_witness_commitment
from test_framework.messages import CTransaction, CTxIn, CTxOut, COutPoint, CTxInWitness
from test_framework.script import CScript, TaprootSignatureHash
from test_framework.key import sign_schnorr

RESERVE=bytes([0x5f,32])+bytes([0x43])*32
SAT=100_000_000

def native_output(encoded):
    b=bytes.fromhex(encoded)
    if b[0]==0:
        assert len(b)==41
        return CTxOut(int.from_bytes(b[1:9],'little'),CScript(b'\x51\x20'+b[9:]))
    assert len(b)==122 and b[0]==1
    return CTxOut(0,CScript(b'\x60\x20'+hashlib.sha256(b).digest()))

def outpoint(txid,index):return COutPoint(int(txid,16),index)
def txid(tx):return tx.txid_hex

class Demo:
    def __init__(self):
        self.run=HERE/'runs'/(datetime.now().strftime('%Y%m%d-%H%M%S')+'-'+secrets.token_hex(3))
        self.run.mkdir(parents=True)
        ports=lab.free_ports(4)
        self.nodes=[{'datadir':str(self.run/f'{name}-node'),'rpc_port':ports[2*i],'p2p_port':ports[2*i+1],'flavor':name} for i,name in enumerate(('stock','upgraded'))]
        self.seeds={n:secrets.token_hex(32) for n in ('alice','bob')}
        lab.save(self.run/'test-seeds.json',self.seeds)
        self.identities={n:lab.wallet('identity',seed=s) for n,s in self.seeds.items()}
        self.report={'scope':'private regtest differential consensus experiment','success':False,'checks':[],'transactions':[],'run':str(self.run)}
        self.reserve=None
        self.coins={'alice':[],'bob':[]}
        lab.save(HERE/'latest.json',{'run':str(self.run),'nodes':self.nodes})

    def save(self):
        lab.save(self.run/'report.json',self.report)
        lab.save(HERE/'report.json',self.report)

    def start(self):
        for n in self.nodes:
            lab.DAEMON=HERE/f'bin/{n["flavor"]}/bitcoind.exe'
            lab.start_node(n)
        self.report['binaries']={n['flavor']:hashlib.sha256((HERE/f'bin/{n["flavor"]}/bitcoind.exe').read_bytes()).hexdigest() for n in self.nodes}
        self.save()

    def block(self,txs=(),parent=None):
        n=self.nodes[0]
        parent=parent or lab.rpc(n,'getbestblockhash')
        header=lab.rpc(n,'getblockheader',parent)
        height=header['height']+1
        cb=create_coinbase(height,script_pubkey=CScript(b'\x51\x20'+bytes.fromhex(self.identities['alice']['owner'])))
        if height==1:cb.vout.append(CTxOut(0,CScript(RESERVE)))
        b=create_block(int(parent,16),cb,ntime=max(int(time.time()),header['time']+1),txlist=list(txs),version=0x20000000)
        add_witness_commitment(b)
        b.solve()
        return b

    def submit(self,b,side_branch=False):
        for n in self.nodes:
            result=lab.rpc(n,'submitblock',b.serialize().hex())
            assert result is None or (side_branch and result=='inconclusive'),(n['flavor'],result)
        assert lab.rpc(self.nodes[0],'getbestblockhash')==lab.rpc(self.nodes[1],'getbestblockhash')

    def build(self,inputs,payments,fee=1000):
        made=lab.wallet('build',inputs=inputs,payments=payments,fee=fee)
        tx=CTransaction()
        tx.vin=[CTxIn(outpoint(self.reserve['txid'],self.reserve['vout']))]+[CTxIn(outpoint(c['txid'],c['vout'])) for c in inputs]
        for i in tx.vin:i.nSequence=0xffffffff
        outputs=[native_output(o) for o in made['outputs']]
        prior=[CTxOut(self.reserve['value'],CScript(RESERVE))]+[native_output(c['output']) for c in inputs]
        reserve_value=sum(o.nValue for o in prior)-sum(o.nValue for o in outputs)-fee
        assert reserve_value>=0
        tx.vout=[CTxOut(reserve_value,CScript(RESERVE))]+outputs
        tx.wit.vtxinwit=[CTxInWitness() for _ in tx.vin]
        tx.wit.vtxinwit[0].scriptWitness.stack=[bytes.fromhex(made['payload']),struct.pack('<I',len(made['outputs']))+b''.join(bytes.fromhex(o) for o in made['outputs'])]
        for i,c in enumerate(inputs,1):
            if bytes.fromhex(c['output'])[0]==1:
                tx.wit.vtxinwit[i].scriptWitness.stack=[bytes.fromhex(c['output'])]
            else:
                sig=sign_schnorr(bytes.fromhex(c['secret']),TaprootSignatureHash(tx,prior,0,input_index=i))
                tx.wit.vtxinwit[i].scriptWitness.stack=[sig]
        return tx,made

    def record(self,tx,made,inputs,label):
        self.reserve={'txid':txid(tx),'vout':0,'value':tx.vout[0].nValue}
        spent={(c['txid'],c['vout']) for c in inputs}
        for name in self.coins:
            self.coins[name]=[c for c in self.coins[name] if (c['txid'],c['vout']) not in spent]
            found=lab.wallet('scan',seed=self.seeds[name],payload=made['payload'],previous=[c['output'] for c in inputs],native_txid=txid(tx),output_offset=1)['coins']
            self.coins[name].extend(found)
        item={'case':label,'txid':txid(tx),'reserve_sats':self.reserve['value'],'fee_sats':1000,'bytes':len(tx.serialize()),'confirmed_on_both':True}
        self.report['transactions'].append(item)
        print(json.dumps(item),flush=True)
        self.save()

    def accept(self,inputs,payments,label):
        tx,made=self.build(inputs,payments)
        self.submit(self.block([tx]))
        self.record(tx,made,inputs,label)
        return tx,made

    def attack(self,label,tx,expected='bad-cnu-reserve',stock_accepts=True):
        parent=lab.rpc(self.nodes[0],'getbestblockhash')
        b=self.block([tx])
        raw=b.serialize().hex()
        stock=lab.rpc(self.nodes[0],'submitblock',raw)
        upgraded=lab.rpc(self.nodes[1],'submitblock',raw)
        if stock_accepts:
            assert stock is None,(label,stock)
            assert lab.rpc(self.nodes[0],'getbestblockhash')==b.hash_hex
        else:assert stock==expected,(label,stock)
        assert upgraded==expected,(label,upgraded)
        assert lab.rpc(self.nodes[1],'getbestblockhash')==parent
        if stock_accepts:lab.rpc(self.nodes[0],'invalidateblock',b.hash_hex)
        assert lab.rpc(self.nodes[0],'getbestblockhash')==parent
        item={'case':label,'block':b.hash_hex,'stock_result':stock,'upgraded_result':upgraded,'stock_tip_restored':True}
        self.report['checks'].append(item)
        print(json.dumps(item),flush=True)
        self.save()

    def execute(self):
        self.start()
        first=self.block()
        self.submit(first)
        coinbase=first.vtx[0]
        self.reserve={'txid':txid(coinbase),'vout':1,'value':0}
        funding=lab.wallet('fund',seed=self.seeds['alice'],txid=txid(coinbase),vout=0,value=50*SAT)
        for _ in range(100):self.submit(self.block())
        print('Both nodes accepted 101 ordinary blocks and matured the bootstrap reserve.',flush=True)
        self.accept([funding],[{'address':self.identities['alice']['address'],'value':10*SAT},{'owner':self.identities['alice']['owner'],'value':40*SAT-1000}],'deposit')
        self.accept(self.coins['alice'].copy(),[{'address':self.identities['bob']['address'],'value':3*SAT},{'address':self.identities['alice']['address'],'value':7*SAT-1000}],'confidential_payment')
        self.accept(self.coins['bob'].copy(),[{'address':self.identities['bob']['address'],'value':2*SAT},{'owner':self.identities['bob']['owner'],'value':SAT-1000}],'transparent_withdrawal')

        bob=self.coins['bob'].copy()
        fraud,_=self.build(bob,[{'address':self.identities['bob']['address'],'value':3*SAT-1000}])
        self.attack('hidden_value_inflation',fraud)
        valid,made=self.build(bob,[{'address':self.identities['alice']['address'],'value':SAT},{'address':self.identities['bob']['address'],'value':SAT-1000}])
        bad=CTransaction(valid)
        tampered=lab.wallet('tamper_proof',payload=made['payload'])
        bad.wit.vtxinwit[0].scriptWitness.stack[0]=bytes.fromhex(tampered['payload'])
        self.attack('malformed_range_proof',bad)
        bad=CTransaction(valid)
        p=bytearray(bad.wit.vtxinwit[0].scriptWitness.stack[0])
        base_size=12+36*len(bob)+4+sum(len(bytes.fromhex(o)) for o in made['outputs'])+12
        p[base_size+4]^=1
        bad.wit.vtxinwit[0].scriptWitness.stack[0]=bytes(p)
        self.attack('forged_ownership_signature',bad)
        bad=CTransaction(valid)
        s=bytearray(bad.vout[1].scriptPubKey);s[-1]^=1;bad.vout[1].scriptPubKey=CScript(s)
        self.attack('native_commitment_mismatch',bad)
        bad=CTransaction(valid)
        meta=bytearray(bad.wit.vtxinwit[1].scriptWitness.stack[0]);meta[-1]^=1
        bad.wit.vtxinwit[1].scriptWitness.stack=[bytes(meta)]
        self.attack('forged_previous_coin_metadata',bad)
        bad=CTransaction(valid)
        bad.wit.vtxinwit[0].scriptWitness.stack=[]
        self.attack('missing_proof_data',bad)
        # Valid zero-valued carrier arithmetic to old nodes; bypasses new ownership.
        bad=CTransaction()
        bad.vin=[CTxIn(outpoint(bob[0]['txid'],bob[0]['vout']))]
        bad.vout=[CTxOut(0,CScript(b'\x51'))]
        self.attack('ordinary_transaction_steals_carrier',bad)
        # Public reserve theft is old-rule valid, but lacks the required confidential burn.
        bad=CTransaction()
        bad.vin=[CTxIn(outpoint(self.reserve['txid'],0))]
        bad.vout=[CTxOut(self.reserve['value']-1000,CScript(b'\x51\x20'+bytes.fromhex(self.identities['alice']['owner'])))]
        self.attack('unauthorized_reserve_withdrawal',bad)

        # Honest reorg: two competing branches, no local invalidation for fork selection.
        parent=lab.rpc(self.nodes[0],'getbestblockhash')
        reserve_before=copy.deepcopy(self.reserve)
        coins_before=copy.deepcopy(self.coins)
        branch_a=self.block([valid]);self.submit(branch_a)
        alternative=self.block(parent=parent);self.submit(alternative,side_branch=True)
        winner=self.block(parent=alternative.hash_hex);self.submit(winner)
        for n in self.nodes:
            assert lab.rpc(n,'getbestblockhash')==winner.hash_hex
            assert lab.rpc(n,'gettxout',reserve_before['txid'],0,False) is not None
            assert lab.rpc(n,'gettxout',txid(valid),0,False) is None
        self.report['reorg']={'orphaned':branch_a.hash_hex,'winning_tip':winner.hash_hex,'reserve_and_carrier_rollback':True}
        self.reserve=reserve_before;self.coins=coins_before
        self.submit(self.block([valid]));self.record(valid,made,bob,'payment_after_reorg')
        duplicate=CTransaction(valid);duplicate.nLockTime=1
        self.attack('double_spend',duplicate,'bad-txns-inputs-missingorspent',False)
        self.report['persistence']={}
        for n in self.nodes:lab.stop_node(n)
        for n in self.nodes:
            lab.DAEMON=HERE/f'bin/{n["flavor"]}/bitcoind.exe'
            lab.start_node(n,extra=('-reindex-chainstate=1',))
            assert lab.rpc(n,'verifychain',4,0)
            assert lab.rpc(n,'gettxout',self.reserve['txid'],0,False) is not None
            self.report['persistence'][n['flavor']]={'reindexed':True,'verifychain':True}
        self.report['final']={'nodes':[{'flavor':n['flavor'],'height':lab.rpc(n,'getblockcount'),'tip':lab.rpc(n,'getbestblockhash')} for n in self.nodes],'reserve_sats':self.reserve['value'],'confidential_balances_sats':lab.balances(self.coins)}
        assert sum(lab.balances(self.coins).values())==self.reserve['value']
        for name,coins in self.coins.items():lab.save(self.run/f'{name}.cache.json',coins)
        for name in self.coins:(self.run/f'{name}.cache.json').unlink()
        for n in self.nodes:
            recovered=self.recover(n)
            for name in self.coins:
                keyed=lambda cs:{(c['txid'],c['vout']):c for c in cs}
                assert keyed(recovered[name])==keyed(self.coins[name])
                lab.save(self.run/f'{name}.cache.json',recovered[name])
        self.report['seed_recovery']={'caches_deleted':True,'exact_records_from_both_chains':True}
        self.report['upstream_commit']='e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6'
        self.report['success']=True
        self.save()
        print(json.dumps({'success':True,'report':str(HERE/'report.json'),'final':self.report['final']},indent=2),flush=True)

    def recover(self,node):
        history={};found={name:{} for name in self.seeds}
        for h in range(1,lab.rpc(node,'getblockcount')+1):
            block=lab.rpc(node,'getblock',lab.rpc(node,'getblockhash',h),2)
            for tx in block['tx']:
                encoded=None
                if tx['vout'] and tx['vout'][0]['scriptPubKey']['hex']==RESERVE.hex():
                    payload,metadata=tx['vin'][0]['txinwitness']
                    b=bytes.fromhex(metadata);count=int.from_bytes(b[:4],'little');at=4;encoded=[]
                    for _ in range(count):
                        size=41 if b[at]==0 else 122
                        encoded.append(b[at:at+size].hex());at+=size
                    assert at==len(b)
                    prev=[history[(i['txid'],i['vout'])] for i in tx['vin'][1:]]
                    for name in found:
                        coins=lab.wallet('scan',seed=self.seeds[name],payload=payload,previous=prev,native_txid=tx['txid'],output_offset=1)['coins']
                        for c in coins:found[name][(c['txid'],c['vout'])]=c
                for i in tx['vin']:
                    if 'txid' in i:
                        op=(i['txid'],i['vout']);history.pop(op,None)
                        for w in found.values():w.pop(op,None)
                for o in tx['vout']:
                    key=(tx['txid'],o['n']);script=bytes.fromhex(o['scriptPubKey']['hex'])
                    if encoded is not None and o['n']>0:history[key]=encoded[o['n']-1]
                    elif len(script)==34 and script[:2]==b'\x51\x20':
                        history[key]=(b'\x00'+struct.pack('<Q',int(Decimal(str(o['value']))*SAT))+script[2:]).hex()
        return {name:list(w.values()) for name,w in found.items()}

if __name__=='__main__':
    # Preserve the original command while running the current protocol experiment.
    import subprocess
    raise SystemExit(subprocess.call([sys.executable,str(HERE/'v2_demo.py')]))
