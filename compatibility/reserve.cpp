// V2 research restrictions over unchanged public Bitcoin amount accounting.
#include <cnu_reserve/reserve.h>
#include <coins.h>
#include <crypto/sha256.h>
#include <hash.h>
#include <primitives/transaction.h>
#include <pubkey.h>
#include <algorithm>
#include <cstdint>
#include <vector>
extern "C" int cnu_verify_v0(const unsigned char*,size_t,const unsigned char*,size_t,const unsigned char*,size_t,uint32_t);
namespace {
using Bytes=std::vector<unsigned char>;
int activation_height=0; // Set once at startup; regtest-only consensus parameter.
constexpr size_t MAX_PAYLOAD=250000,MAX_META=500000,MAX_IO=256;
constexpr CAmount MAX_FRAGMENT=10*COIN; // Research bound, not a production parameter.
const Bytes MAGIC{0x50,'C','N','U','2'};
void U32(Bytes& b,uint32_t n){for(int i=0;i<4;++i)b.push_back(n>>(8*i));}
void U64(Bytes& b,uint64_t n){for(int i=0;i<8;++i)b.push_back(n>>(8*i));}
bool Carrier(const CScript& s){return s.size()==34 && s[0]==OP_16 && s[1]==32;}
bool Reserve(const CScript& s){static const Bytes tag(32,0x44);return s.size()==34 && s[0]==OP_15 && s[1]==32 && std::equal(tag.begin(),tag.end(),s.begin()+2);}
bool P2TR(const CScript& s){return s.size()==34 && s[0]==OP_1 && s[1]==32;}
bool Matches(const Bytes& meta,const CTxOut& out){
    const auto& s=out.scriptPubKey;
    if(meta.size()==41 && meta[0]==0){
        uint64_t value=0;for(int i=0;i<8;++i)value|=uint64_t(meta[1+i])<<(8*i);
        return value<=MAX_MONEY && out.nValue==value && P2TR(s) && std::equal(meta.begin()+9,meta.end(),s.begin()+2);
    }
    if(meta.size()==122 && meta[0]==1 && out.nValue==0 && Carrier(s)){
        unsigned char h[32];CSHA256().Write(meta.data(),meta.size()).Finalize(h);
        return std::equal(h,h+32,s.begin()+2);
    }
    return false;
}
bool Read32(const Bytes& b,size_t& at,uint32_t& n){
    if(at>b.size() || b.size()-at<4)return false;
    n=0;for(int i=0;i<4;++i)n|=uint32_t(b[at++])<<(8*i);return true;
}
bool Envelope(const Bytes& b,Bytes& payload,std::vector<Bytes>& outputs){
    if(b.size()<MAGIC.size() || b.size()>MAX_PAYLOAD+MAX_META+13 || !std::equal(MAGIC.begin(),MAGIC.end(),b.begin()))return false;
    size_t at=MAGIC.size();uint32_t size,count;
    if(!Read32(b,at,size) || size>MAX_PAYLOAD || size>b.size()-at)return false;
    payload.assign(b.begin()+at,b.begin()+at+size);at+=size;
    if(!Read32(b,at,count) || count>MAX_IO || b.size()-at>MAX_META)return false;
    for(uint32_t i=0;i<count;++i){
        if(at>=b.size())return false;
        size_t len=b[at]==0?41:b[at]==1?122:0;
        if(!len || len>b.size()-at)return false;
        outputs.emplace_back(b.begin()+at,b.begin()+at+len);at+=len;
    }
    return at==b.size();
}
}
void cnu_reserve::SetActivationHeight(int height){activation_height=height;}
int cnu_reserve::ActivationHeight(){return activation_height;}
bool cnu_reserve::Active(int height){return height>=activation_height;}
unsigned cnu_reserve::Work(const CTransaction& tx,const CCoinsViewCache& coins,int height){
    if(!Touches(tx,coins,height))return 0;
    unsigned work=1; // Exact-zero excess proof.
    for(const auto& out:tx.vout)if(Carrier(out.scriptPubKey))work+=2;
    for(const auto& in:tx.vin){const auto& coin=coins.AccessCoin(in.prevout);if(coin.nHeight>=activation_height && Reserve(coin.out.scriptPubKey))continue;work+=(coin.nHeight>=activation_height && Carrier(coin.out.scriptPubKey))?2:1;}
    return work;
}
bool cnu_reserve::Touches(const CTransaction& tx,const CCoinsViewCache& coins,int height){
    if(!Active(height))return false;
    for(const auto& in:tx.vin){const auto& coin=coins.AccessCoin(in.prevout);const auto& s=coin.out.scriptPubKey;if(coin.nHeight>=activation_height && (Reserve(s)||Carrier(s)))return true;}
    for(const auto& o:tx.vout)if(Reserve(o.scriptPubKey)||Carrier(o.scriptPubKey))return true;
    return false;
}
bool cnu_reserve::Coinbase(const CTransaction& tx,int height){
    if(!Active(height))return true;
    // No special bootstrap: deposits create backed fragments after normal maturity.
    for(const auto& o:tx.vout)if(Reserve(o.scriptPubKey)||Carrier(o.scriptPubKey))return false;
    return true;
}
bool cnu_reserve::Check(const CTransaction& tx,const CCoinsViewCache& coins,int height,long long fee){
    if(!Touches(tx,coins,height))return true;
    if(Work(tx,coins,height)>MAX_TX_WORK)return false;
    if(tx.version!=2 || tx.nLockTime!=0 || tx.vin.empty() || tx.vin.size()>MAX_IO || tx.vout.empty() || tx.vout.size()>MAX_IO || fee<0 || fee>MAX_MONEY)return false;
    size_t ri=0,ro=0;CAmount reserve_in=0,reserve_out=0,smallest=MAX_MONEY;
    while(ri<tx.vin.size() && coins.AccessCoin(tx.vin[ri].prevout).nHeight>=activation_height && Reserve(coins.AccessCoin(tx.vin[ri].prevout).out.scriptPubKey)){
        const auto value=coins.AccessCoin(tx.vin[ri].prevout).out.nValue;
        if(value<=0 || value>MAX_FRAGMENT)return false;
        reserve_in+=value;smallest=std::min(smallest,value);++ri;
    }
    while(ro<tx.vout.size() && Reserve(tx.vout[ro].scriptPubKey)){
        const auto value=tx.vout[ro].nValue;if(value<=0 || value>MAX_FRAGMENT)return false;
        reserve_out+=value;++ro;
    }
    // Prevent spending unrelated backing just to churn or consolidate the pool.
    if(ri && (reserve_in<=reserve_out || reserve_in-smallest>=reserve_in-reserve_out))return false;
    if(ri==tx.vin.size() || ro==tx.vout.size())return false;
    for(const auto& in:tx.vin)if(!in.scriptSig.empty() || in.nSequence!=CTxIn::SEQUENCE_FINAL)return false;
    for(size_t i=0;i<ri;++i)if(!tx.vin[i].scriptWitness.IsNull())return false;
    const auto& first=tx.vin[ri];const auto& first_out=coins.AccessCoin(first.prevout).out;
    const auto& stack=first.scriptWitness.stack;
    if((Carrier(first_out.scriptPubKey) && stack.size()!=3) || (P2TR(first_out.scriptPubKey) && stack.size()!=2) || stack.empty())return false;
    Bytes payload;std::vector<Bytes> outputs;
    if(!Envelope(stack.back(),payload,outputs) || outputs.size()+ro!=tx.vout.size())return false;
    Bytes base{'C','N','U','0'};U32(base,0x43505554);U32(base,tx.vin.size()-ri);
    Bytes prev;U32(prev,tx.vin.size()-ri);
    const uint256 outer=(TaggedHash("CNU/outer/v2") << TX_NO_WITNESS(tx)).GetSHA256();
    for(size_t i=ri;i<tx.vin.size();++i){
        const auto& in=tx.vin[i];const auto& out=coins.AccessCoin(in.prevout).out;
        for(const auto byte:in.prevout.hash)base.push_back(std::to_integer<unsigned char>(byte));
        U32(base,in.prevout.n);Bytes meta;
        if(Carrier(out.scriptPubKey) && coins.AccessCoin(in.prevout).nHeight>=activation_height){
            const auto& w=in.scriptWitness.stack;
            if(w.size()!=(i==ri?3:2) || w[0].size()!=122 || w[1].size()!=64)return false;
            meta=w[0];if(!Matches(meta,out))return false;
            const XOnlyPubKey key{std::span<const unsigned char>{meta.data()+1,32}};
            if(!key.VerifySchnorr(outer,w[1]))return false;
        }else{
            if(!P2TR(out.scriptPubKey) || !MoneyRange(out.nValue))return false;
            const auto& w=in.scriptWitness.stack;
            if(w.size()!=(i==ri?2:1) || w[0].size()!=64)return false;
            meta.push_back(0);U64(meta,out.nValue);meta.insert(meta.end(),out.scriptPubKey.begin()+2,out.scriptPubKey.end());
        }
        prev.insert(prev.end(),meta.begin(),meta.end());
    }
    U32(base,outputs.size());
    for(size_t i=0;i<outputs.size();++i){
        if(!Matches(outputs[i],tx.vout[ro+i]))return false;
        base.insert(base.end(),outputs[i].begin(),outputs[i].end());
    }
    U64(base,fee);U32(base,tx.nLockTime);
    return cnu_verify_v0(payload.data(),payload.size(),base.data(),base.size(),prev.data(),prev.size(),height)==1;
}
