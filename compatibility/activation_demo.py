"""Real-node fixed-height activation regression; not a deployment state machine."""
import copy
import hashlib
from v2_demo import Demo, lab, HERE, SAT, FEE, RESERVE, CTransaction, CTxIn, CTxOut, CTxInWitness, CScript, outpoint, txid, sign_schnorr, TaprootSignatureHash, add_witness_commitment

class ActivationDemo(Demo):
    activation_height=105
    def save(self):
        lab.save(self.run/'report.json',self.report)
        lab.save(HERE/'report-activation.json',self.report)
    def start(self):
        for n in self.nodes:
            lab.DAEMON=self.binary(n)
            extra=() if n['flavor']=='stock' else (f'-cnuactivationheight={self.activation_height}',)
            lab.start_node(n,extra=('-acceptnonstdtxn=0',*extra))
        self.report['binaries']={n['flavor']:hashlib.sha256(self.binary(n).read_bytes()).hexdigest() for n in self.nodes}
        self.report['scope']='fixed-height activation, historical namespace and rollback regression'
    def ordinary(self,coin):
        tx=CTransaction();tx.vin=[CTxIn(outpoint(coin['txid'],coin['vout']))]
        tx.vout=[CTxOut(coin['value']-FEE,CScript(b'\x51\x20'+bytes.fromhex(self.identities['alice']['owner'])))]
        tx.wit.vtxinwit=[CTxInWitness()]
        prev=CTxOut(coin['value'],CScript(b'\x51\x20'+bytes.fromhex(self.identities['alice']['owner'])))
        tx.wit.vtxinwit[0].scriptWitness.stack=[sign_schnorr(bytes.fromhex(coin['secret']),TaprootSignatureHash(tx,[prev],0,input_index=0))]
        return tx
    def execute(self):
        self.start();funds={};first=None
        for h in range(1,104):
            b=self.block()
            if h==1:
                b.vtx[0].vout[0].nValue-=SAT
                b.vtx[0].vout.insert(1,CTxOut(SAT,CScript(RESERVE)))
                b.vtx[0].vout.insert(2,CTxOut(0,CScript(b'\x60\x20'+bytes([7])*32)))
                b.hashMerkleRoot=b.calc_merkle_root();b.solve();first=b
            self.submit(b)
            if h in (2,3,4):funds[h]=self.transparent_coin(b.vtx[0],0,'bob' if h==2 else 'alice')
        # A pre-boundary ordinary mempool entry is conservatively evicted when
        # the next-block rules change. Re-admission must use the new context.
        pending=self.ordinary(funds[4])
        for n in self.nodes[1:]:lab.rpc(n,'sendrawtransaction',pending.serialize().hex())
        ghost,gm=self.build([funds[2]],[{'address':self.identities['bob']['address'],'value':2*SAT},{'owner':self.identities['bob']['owner'],'value':48*SAT-FEE}])
        pre=self.block([ghost]);self.submit(pre)
        for n in self.nodes[1:]:assert not lab.rpc(n,'getrawmempool')
        self.report['boundary_mempool_eviction']=True
        deposit,dm=self.build([funds[3]],[{'address':self.identities['alice']['address'],'value':2*SAT},{'owner':self.identities['alice']['owner'],'value':48*SAT-FEE}])
        claim=lab.wallet('scan',seed=self.seeds['alice'],payload=dm['payload'],previous=[c['output'] for c in dm['inputs']],native_txid=txid(deposit),output_offset=dm['offset'])['coins'][0]
        backing={'txid':txid(deposit),'vout':0,'value':2*SAT}
        child,cm=self.build([claim],[{'address':self.identities['alice']['address'],'value':2*SAT-FEE}],reserves=[backing])
        active=self.block([deposit,child]);self.submit(active)
        self.record(deposit,dm,'activation_deposit');self.record(child,cm,'same_block_spend')
        self.report['activation_same_block_spend']=True
        fake=lab.wallet('scan',seed=self.seeds['bob'],payload=gm['payload'],previous=[c['output'] for c in gm['inputs']],native_txid=txid(ghost),output_offset=gm['offset'])['coins'][0]
        forged,fm=self.build([fake],[{'address':self.identities['bob']['address'],'value':2*SAT-FEE}],reserves=list(self.reserves.values()))
        self.attack('historical_claim_cannot_redeem_new_backing',forged)
        # Both preactivation lookalikes retain historical public spending rules.
        old=CTransaction();old.vin=[CTxIn(outpoint(txid(first.vtx[0]),i)) for i in (1,2)]
        old.vout=[CTxOut(SAT-FEE,CScript(b'\x51\x20'+bytes.fromhex(self.identities['alice']['owner'])))]
        oldblock=self.block([old]);self.submit(oldblock)
        oldblock=self.block();self.submit(oldblock)
        self.report['historical_reserve_and_carrier_public_spends']=True
        for n in self.nodes:
            lab.rpc(n,'invalidateblock',pre.hash_hex)
            assert lab.rpc(n,'getblockcount')==103
        for n in self.nodes[1:]:
            pool=lab.rpc(n,'getrawmempool')
            assert txid(deposit) not in pool and txid(child) not in pool
            result=lab.rpc(n,'testmempoolaccept',[deposit.serialize().hex()])[0]
            assert not result['allowed'] and result['reject-reason']=='cnu-inactive',result
        for n in self.nodes:
            lab.rpc(n,'reconsiderblock',pre.hash_hex)
            assert lab.rpc(n,'getbestblockhash')==oldblock.hash_hex
        self.report['rollback_below_activation_and_reconnect']=True
        for n in self.nodes:lab.stop_node(n)
        for n in self.nodes:
            lab.DAEMON=self.binary(n)
            extra=() if n['flavor']=='stock' else (f'-cnuactivationheight={self.activation_height}',)
            lab.start_node(n,extra=('-reindex-chainstate=1','-acceptnonstdtxn=0',*extra))
            assert lab.rpc(n,'verifychain',4,0)
            found=self.recover(n)
            assert found['bob']==[] # Historical fake claim never enters wallet balance.
            assert {(c['txid'],c['vout']) for c in found['alice']}=={(c['txid'],c['vout']) for c in self.coins['alice']}
        self.report['historical_seed_recovery_and_reindex']=True
        self.report['activation_height']=self.activation_height
        self.report['success']=True;self.save()
        print(self.report,flush=True)

if __name__=='__main__':
    demo=ActivationDemo()
    try:demo.execute()
    except Exception as e:demo.report['failure']=str(e);demo.save();raise
    finally:
        for n in demo.nodes:lab.stop_node(n)



