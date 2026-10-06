#include <stdlib.h>
#include <string.h>
#include "secp256k1.h"
#include "secp256k1_generator.h"
#include "secp256k1_commitment.h"
#include "secp256k1_bulletproofs.h"
typedef struct { secp256k1_context *ctx; secp256k1_scratch_space *scratch; secp256k1_bulletproof_generators *gens; secp256k1_generator h; } bp;
void cnu_bp_free(bp *b) { if (!b) return; if(b->gens) secp256k1_bulletproof_generators_destroy(b->ctx,b->gens); if(b->scratch) secp256k1_scratch_space_destroy(b->scratch); if(b->ctx) secp256k1_context_destroy(b->ctx); free(b); }
bp *cnu_bp_new(const unsigned char *h) { bp *b=calloc(1,sizeof(bp)); if(!b) return NULL; b->ctx=secp256k1_context_create(SECP256K1_CONTEXT_SIGN|SECP256K1_CONTEXT_VERIFY); if(!b->ctx) {cnu_bp_free(b);return NULL;} if(!secp256k1_generator_parse(b->ctx,&b->h,h)) {cnu_bp_free(b);return NULL;} b->scratch=secp256k1_scratch_space_create(b->ctx,64*1024*1024); b->gens=secp256k1_bulletproof_generators_create(b->ctx,&secp256k1_generator_const_g,8192); if(!b->scratch||!b->gens) {cnu_bp_free(b);return NULL;} return b; }
int cnu_bp_prove(bp *b,const uint64_t *values,const unsigned char *blinds,size_t n,const unsigned char *bind,const unsigned char *nonce,unsigned char *proof,size_t *len,unsigned char *commits) {
 const unsigned char *ptrs[64]; size_t i; secp256k1_pedersen_commitment c;
 if(n<2||n>64||(n&(n-1))) return 0;
 for(i=0;i<n;i++){ptrs[i]=blinds+32*i; if(!secp256k1_pedersen_commit(b->ctx,&c,ptrs[i],values[i],&b->h,&secp256k1_generator_const_g)) return 0; if(!secp256k1_pedersen_commitment_serialize(b->ctx,commits+33*i,&c)) return 0;}
 return secp256k1_bulletproof_rangeproof_prove(b->ctx,b->scratch,b->gens,proof,len,NULL,NULL,NULL,values,NULL,ptrs,NULL,n,&b->h,64,nonce,NULL,bind,32,NULL);
}
int cnu_bp_verify(bp *b,const unsigned char *proof,size_t len,const unsigned char *commits,size_t n,const unsigned char *bind) {
 secp256k1_pedersen_commitment c[64]; unsigned char check[33]; size_t i;
 if(n<2||n>64||(n&(n-1))||len>4096||len<1) return 0;
 for(i=0;i<n;i++){if(!secp256k1_pedersen_commitment_parse(b->ctx,&c[i],commits+33*i)) return 0; if(!secp256k1_pedersen_commitment_serialize(b->ctx,check,&c[i])||memcmp(check,commits+33*i,33)) return 0;}
 return secp256k1_bulletproof_rangeproof_verify(b->ctx,b->scratch,b->gens,proof,len,NULL,c,n,64,&b->h,bind,32);
}
