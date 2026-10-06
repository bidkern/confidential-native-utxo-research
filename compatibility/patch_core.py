"""Apply the research restrictions to a clean, pinned Core checkout, after stock build."""
from pathlib import Path
import shutil

HERE=Path(__file__).resolve().parent
CORE=HERE/'bitcoin-core'

def replace(file, old, new):
    path=CORE/file
    text=path.read_text()
    if text.count(old)!=1:raise RuntimeError(f'Expected one clean anchor: {file}: {old}')
    path.write_text(text.replace(old,new))

def main():
    dest=CORE/'src/cnu_reserve'
    dest.mkdir(exist_ok=True)
    for name in ('reserve.h','reserve.cpp'):shutil.copyfile(HERE/name,dest/name)
    replace('src/consensus/tx_verify.cpp','#include <consensus/tx_verify.h>', '#include <consensus/tx_verify.h>\n#include <cnu_reserve/reserve.h>')
    replace('src/consensus/tx_verify.cpp','    txfee = txfee_aux;', '''    if (!cnu_reserve::Check(tx, inputs, nSpendHeight, txfee_aux))
        return state.Invalid(tx.HasWitness() ? TxValidationResult::TX_WITNESS_MUTATED : TxValidationResult::TX_WITNESS_STRIPPED, "bad-cnu-reserve");
    txfee = txfee_aux;''')
    replace('src/validation.cpp','#include <validation.h>','#include <validation.h>\n#include <cnu_reserve/reserve.h>')
    replace('src/validation.cpp','    constexpr script_verify_flags scriptVerifyFlags = STANDARD_SCRIPT_VERIFY_FLAGS;', '''    script_verify_flags scriptVerifyFlags = STANDARD_SCRIPT_VERIFY_FLAGS;
    // Scoped extension policy; CheckTxInputs already enforced every extension rule.
    if (cnu_reserve::Touches(tx, m_view))
        scriptVerifyFlags &= ~SCRIPT_VERIFY_DISCOURAGE_UPGRADABLE_WITNESS_PROGRAM;''')
    replace('src/validation.cpp','        nInputs += tx.vin.size();','''        if (tx.IsCoinBase() && !cnu_reserve::Coinbase(tx, pindex->nHeight))
            return state.Invalid(BlockValidationResult::BLOCK_CONSENSUS, "bad-cnu-bootstrap");
        nInputs += tx.vin.size();''')
    replace('src/init.cpp','    if (chainparams.GetChainType() == ChainType::MAIN) {','''    if (chainparams.GetChainType() != ChainType::REGTEST)
        return InitError(_("CNU reserve research binary is restricted to regtest."));
    if (chainparams.GetChainType() == ChainType::MAIN) {''')
    replace('src/CMakeLists.txt','add_library(bitcoin_node STATIC EXCLUDE_FROM_ALL','''add_library(bitcoin_node STATIC EXCLUDE_FROM_ALL
  cnu_reserve/reserve.cpp''')
    with (CORE/'src/CMakeLists.txt').open('a') as f:
        f.write('\ntarget_link_libraries(bitcoin_node PRIVATE "${CNU_RUST_LIBRARY}" $<$<PLATFORM_ID:Windows>:ntdll;userenv;bcrypt;ws2_32>)\n')
    patch_policy()
    patch_activation()
    patch_mining()

def patch_policy():
    replace('src/policy/policy.cpp','#include <policy/policy.h>','#include <policy/policy.h>\n#include <cnu_reserve/reserve.h>')
    replace('src/policy/policy.cpp','    return (txout.nValue < GetDustThreshold(txout, dustRelayFeeIn));','''    if (cnu_reserve::IsCarrierScript(txout.scriptPubKey)) return false;
    return (txout.nValue < GetDustThreshold(txout, dustRelayFeeIn));''')
    replace('src/policy/policy.cpp','        } else if (whichType == TxoutType::WITNESS_UNKNOWN) {','''        } else if (whichType == TxoutType::WITNESS_UNKNOWN &&
                   !cnu_reserve::IsCarrierScript(prev.scriptPubKey) &&
                   !cnu_reserve::IsReserveScript(prev.scriptPubKey)) {''')
    replace('src/policy/policy.cpp','''                // Annexes are nonstandard as long as no semantics are defined for them.
                return false;''','''                const auto& annex = stack.back();
                const std::array<unsigned char,5> magic{0x50,'C','N','U','2'};
                if (!cnu_reserve::Touches(tx, mapInputs) || stack.size()!=2 || annex.size()<5 ||
                    !std::equal(magic.begin(),magic.end(),annex.begin())) return false;
                stack = stack.first(1); // The scoped envelope was checked in CheckTxInputs.''')

def patch_activation():
    replace('src/init.cpp','#include <init.h>','#include <init.h>\n#include <cnu_reserve/reserve.h>')
    replace('src/init.cpp','    argsman.AddArg("-fastprune",','    argsman.AddArg("-cnuactivationheight=<n>", "Regtest-only CNU activation height (default 0; do not change for an existing datadir)", ArgsManager::ALLOW_ANY | ArgsManager::DEBUG_ONLY, OptionsCategory::DEBUG_TEST);\n    argsman.AddArg("-fastprune",')
    replace('src/init.cpp','    // Prevent setting deployment parameters on mainnet.','''    const auto cnu_height = args.GetIntArg("-cnuactivationheight", 0);
    if (cnu_height < 0 || cnu_height > 2147483646) return InitError(Untranslated("Invalid CNU activation height"));
    cnu_reserve::SetActivationHeight(static_cast<int>(cnu_height));
    // Prevent setting deployment parameters on mainnet.''')
    replace('src/validation.cpp','    if (cnu_reserve::Touches(tx, m_view))','    if (cnu_reserve::Touches(tx, m_view, m_active_chainstate.m_chain.Height() + 1))')
    replace('src/validation.cpp','    if (!Consensus::CheckTxInputs(tx, state, m_view,','''    if (!cnu_reserve::Active(m_active_chainstate.m_chain.Height() + 1) && cnu_reserve::Touches(tx, m_view))
        return state.Invalid(TxValidationResult::TX_NOT_STANDARD, "cnu-inactive");
    if (!Consensus::CheckTxInputs(tx, state, m_view,''')
    replace('src/validation.cpp','    int nInputs = 0;', '    int nInputs = 0;\n    unsigned int cnu_work = 0;')
    replace('src/validation.cpp','        nInputs += tx.vin.size();','''        cnu_work += cnu_reserve::Work(tx, view, pindex->nHeight);
        if (cnu_work > cnu_reserve::MAX_BLOCK_WORK)
            return state.Invalid(BlockValidationResult::BLOCK_CONSENSUS, "bad-cnu-block-work");
        nInputs += tx.vin.size();''')
    for anchor, height in [('    m_chain.SetTip(*pindexNew);','pindexNew->nHeight + 1'),('    m_chain.SetTip(*pindexDelete->pprev);','pindexDelete->nHeight + 1')]:
        replace('src/validation.cpp',anchor,anchor+f'''
    // Conservative research policy: discard cached mempool context at the boundary.
    if (m_mempool && {height} == cnu_reserve::ActivationHeight()) {{
        LOCK(m_mempool->cs);
        m_mempool->removeForReorg(m_chain, [](auto) {{ return true; }});
    }}''')

def patch_mining():
    replace('src/node/miner.h','    uint64_t nBlockWeight;', '    uint64_t nBlockWeight;\n    unsigned cnu_block_work{0};')
    replace('src/node/miner.cpp','#include <node/miner.h>', '#include <node/miner.h>\n#include <cnu_reserve/reserve.h>')
    replace('src/node/miner.cpp','    nBlockWeight = *Assert(m_options.block_reserved_weight);','    nBlockWeight = *Assert(m_options.block_reserved_weight);\n    cnu_block_work = 0;')
    replace('src/node/miner.cpp','''    for (const auto tx : txs) {
        if (!IsFinalTx''','''    CCoinsViewMemPool mempool_view{&m_chainstate.CoinsTip(), *Assert(m_mempool)};
    const CCoinsViewCache view{&mempool_view};
    unsigned work = cnu_block_work;
    for (const auto tx : txs) {
        work += cnu_reserve::Work(tx.get().GetTx(), view, nHeight);
        if (work > cnu_reserve::MAX_BLOCK_WORK) return false;
        if (!IsFinalTx''')
    replace('src/node/miner.cpp','    nBlockWeight += entry.GetTxWeight();','''    CCoinsViewMemPool mempool_view{&m_chainstate.CoinsTip(), *Assert(m_mempool)};
    const CCoinsViewCache view{&mempool_view};
    cnu_block_work += cnu_reserve::Work(entry.GetTx(), view, nHeight);
    nBlockWeight += entry.GetTxWeight();''')

if __name__=='__main__':main()
