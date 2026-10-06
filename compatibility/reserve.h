#pragma once
#include <script/script.h>
#include <algorithm>
#include <limits>
class CTransaction;
class CCoinsViewCache;
namespace cnu_reserve {
inline bool IsCarrierScript(const CScript& s) { return s.size()==34 && s[0]==OP_16 && s[1]==32; }
inline bool IsReserveScript(const CScript& s) { return s.size()==34 && s[0]==OP_15 && s[1]==32 && std::all_of(s.begin()+2,s.end(),[](auto b){return b==0x44;}); }
void SetActivationHeight(int);
int ActivationHeight();
bool Active(int);
constexpr unsigned MAX_TX_WORK=128, MAX_BLOCK_WORK=256;
unsigned Work(const CTransaction&, const CCoinsViewCache&, int);
bool Touches(const CTransaction&, const CCoinsViewCache&, int height=std::numeric_limits<int>::max());
bool Check(const CTransaction&, const CCoinsViewCache&, int, long long);
bool Coinbase(const CTransaction&, int);
}
