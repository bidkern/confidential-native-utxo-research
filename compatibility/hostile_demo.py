"""Cold block-work budget, near-full block and unique-invalid traffic experiment."""
import time
import statistics
from v2_demo import *

class HostileDemo(Demo):
    def save(self):
        lab.save(self.run/'report.json',self.report);lab.save(HERE/'report-hostile.json',self.report)
    def plain(self,coin,values,padding=0):
        owner=bytes.fromhex(self.identities['alice']['owner'])
        tx=CTransaction();tx.vin=[CTxIn(outpoint(coin['txid'],coin['vout']))]
        tx.vout=[CTxOut(v,CScript(b'\x51\x20'+owner)) for v in values]
        if padding:tx.vout.append(CTxOut(0,CScript(b'\x6a'+bytes(padding))))
        tx.wit.vtxinwit=[CTxInWitness()]
        tx.wit.vtxinwit[0].scriptWitness.stack=[sign_schnorr(bytes.fromhex(coin['secret']),TaprootSignatureHash(tx,[native_output(coin['output'])],0,input_index=0))]
        return tx
    def execute(self):
        self.start();fund=None
        for h in range(1,104):
            b=self.block();self.submit(b)
            if h==1:fund=self.transparent_coin(b.vtx[0],0,'alice')
        split=self.plain(fund,[5*SAT]*8+[10*SAT-FEE]);self.submit(self.block([split]))
        coins=[self.transparent_coin(split,i,'alice') for i in range(8)]
        txs=[];made_txs=[]
        for c in coins[:6]:
            values=[(5*SAT-FEE)//24]*23;values.append(5*SAT-FEE-sum(values))
            tx,made=self.build([c],[{'address':self.identities['alice']['address'],'value':v} for v in values])
            txs.append(tx);made_txs.append(made)
        # First submit the honest cold near-full block, so its timings are not
        # polluted by verification of a previous over-budget candidate.
        filler=self.plain(coins[6],[5*SAT-FEE],1)
        b=self.block(txs[:5]+[filler]);padding=(3_999_000-b.get_weight())//4
        filler=self.plain(coins[6],[5*SAT-FEE],padding)
        b=self.block(txs[:5]+[filler]);assert 3_990_000<=b.get_weight()<=4_000_000
        times={}
        for n in self.nodes:
            begin=time.perf_counter();result=lab.rpc(n,'submitblock',b.serialize().hex());times[n['flavor']]=(time.perf_counter()-begin)*1000
            assert result in (None,'duplicate'),result
        self.report['near_full_cold_block']={'weight':b.get_weight(),'confidential_outputs':120,'work_units':250,'rpc_ms':times}
        # A sibling with six individually valid deposits exceeds 256 units.
        over=self.block(txs,parent=b.hashPrevBlock.to_bytes(32,'big').hex())
        for n in self.nodes:lab.rpc(n,'invalidateblock',b.hash_hex)
        for tx in txs:
            if txid(tx) not in lab.rpc(self.nodes[1],'getrawmempool'):self.relay(tx)
        template=lab.rpc(self.nodes[1],'getblocktemplate',{'rules':['segwit']})
        assert 0<len(template['transactions'])<=5,template
        self.report['block_assembler_work_budget']={'available':6,'selected':len(template['transactions']),'maximum':5}
        for n in self.nodes:
            result=lab.rpc(n,'submitblock',over.serialize().hex())
            if n['flavor']=='stock':assert result in (None,'inconclusive'),result
            else:assert result in ('bad-cnu-block-work','duplicate-invalid'),result
        lab.rpc(self.nodes[0],'invalidateblock',over.hash_hex)
        for n in self.nodes:lab.rpc(n,'reconsiderblock',b.hash_hex)
        self.report['over_budget_block_rejected']={'units':300,'limit':256,'stock_valid':True}
        self.submit(self.block([txs[5]]))
        for tx,made in zip(txs,made_txs):self.record(tx,made,'load_deposit')
        contested=list(self.reserves.values())[0];claims=self.coins['alice'][:12];withdrawals=[];retry_counts=[]
        for i,claim in enumerate(claims):
            fee=FEE+i*100
            payments=[{'address':self.identities['alice']['address'],'value':claim['value']-10000-fee},{'owner':self.identities['alice']['owner'],'value':10000}]
            tx,made,retries=self.submit_with_reselection([claim],payments,[contested],fee=fee)
            withdrawals.append(tx);retry_counts.append(len(retries));self.record(tx,made,'contended_withdrawal')
        self.submit(self.block(withdrawals))
        self.report['exit_contention']={'independent_claims':12,'initially_selected_same_backing':True,'retry_counts':retry_counts,'all_confirmed':True,'fee_sats':[FEE+i*100 for i in range(12)],'limitation':'One wallet controls different claims; a bounded chain of replacement backing, not multi-user fairness or starvation resistance.'}
        # Unique proofs defeat successful-proof caching. Last-byte mutation
        # damages the excess proof, after output range proofs were processed.
        samples=[];invalids=[]
        for i in range(20):
            tx,made=self.build([coins[7]],[{'address':self.identities['alice']['address'],'value':5*SAT-FEE-i-1000},{'owner':self.identities['alice']['owner'],'value':i+1000}])
            payload=bytearray.fromhex(made['payload']);payload[-1]^=1
            tx.wit.vtxinwit[0].scriptWitness.stack[-1]=envelope(payload.hex(),made['outputs'])
            start=time.perf_counter();r=lab.rpc(self.nodes[1],'testmempoolaccept',[tx.serialize().hex()])[0]
            samples.append((time.perf_counter()-start)*1000)
            assert not r['allowed'] and r['reject-reason']=='bad-cnu-reserve',r
            invalids.append(tx)
        network=NetworkThread();network.start();peer=P2PInterface()
        try:
            lab.wait(lambda:NetworkThread.network_event_loop is not None,'test P2P loop')
            peer.peer_connect(dstaddr='127.0.0.1',dstport=self.nodes[2]['p2p_port'],net='regtest',timeout_factor=1,supports_v2_p2p=False,send_version=True)()
            peer.wait_until(lambda:peer.is_connected,check_connected=False);peer.wait_for_verack()
            start=time.perf_counter()
            for tx in invalids:peer.send_and_ping(msg_tx(tx))
            elapsed=(time.perf_counter()-start)*1000
            assert peer.is_connected
            assert not lab.rpc(self.nodes[2],'getrawmempool')
            self.report['unique_invalid_p2p_burst']={'count':len(invalids),'elapsed_ms':elapsed,'peer_responsive':True,'accepted':0}
            sustained_start=time.perf_counter();count=0
            while time.perf_counter()-sustained_start<60:
                amount=2000+count
                tx,made=self.build([coins[7]],[{'address':self.identities['alice']['address'],'value':5*SAT-FEE-amount},{'owner':self.identities['alice']['owner'],'value':amount}])
                payload=bytearray.fromhex(made['payload']);payload[-1]^=1
                tx.wit.vtxinwit[0].scriptWitness.stack[-1]=envelope(payload.hex(),made['outputs'])
                peer.send_and_ping(msg_tx(tx),timeout=10);count+=1
            assert peer.is_connected and not lab.rpc(self.nodes[2],'getrawmempool')
            self.report['sustained_unique_invalid_p2p']={'seconds':time.perf_counter()-sustained_start,'count':count,'accepted':0,'peer_responsive':True,'limitation':'Single-peer source-limited generation with ping round trips; not a saturation or multi-peer DoS bound.'}

        finally:
            peer.peer_disconnect()
            peer.wait_until(lambda:not peer.is_connected,check_connected=False,timeout=10)
            network.close(timeout=10)
        self.report['unique_invalid_rpc']={'count':len(samples),'median_ms':statistics.median(samples),'max_ms':max(samples),'samples_ms':samples}
        self.report['limitations']='Single-machine bounded burst, not sustained saturation or a worst-case CPU guarantee; proof-unit constants are provisional.'
        self.report['success']=True;self.save();print(self.report,flush=True)

if __name__=='__main__':
    demo=HostileDemo()
    try:demo.execute()
    except Exception as e:demo.report['failure']=str(e);demo.save();raise
    finally:
        for n in demo.nodes:lab.stop_node(n)
