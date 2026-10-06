"""Measured fee scenarios and explicitly hypothetical aggregated-proof wire sizes."""
import json, math
from pathlib import Path
ROOT=Path(__file__).resolve().parent

def projections(samples):
    rows=[]
    for s in samples:
        outputs=s['confidential_outputs']
        # Two bounded values per output preserve the MAX_MONEY complement check.
        values=1 << (2*outputs-1).bit_length()
        rounds=(64*values).bit_length()-1
        illustrative_bytes=33*(2*rounds+4)+32*5
        saved=2*4166*outputs-illustrative_bytes
        # Keep all other overhead; account only for witness proof bytes.
        projected=math.ceil(s['vbytes']-saved/4)
        rows.append({'outputs':outputs,'measured_vbytes':s['vbytes'],
          'measured_cold_rpc_ms':s['valid_testmempoolaccept_ms'],
          'fees_sats':{str(rate):s['vbytes']*rate for rate in (1,5,10)},
          'hypothetical_aggregate_values':values,
          'hypothetical_secp_bulletproof_bytes':illustrative_bytes,
          'hypothetical_vbytes':projected,
          'hypothetical_fees_sats':{str(rate):projected*rate for rate in (1,5,10)}})
    return {'status':'Wire arithmetic scenario, NOT an implemented or benchmarked replacement',
      'unchanged_one_input_two_output_work_units':7,
      'unchanged_max_payments_per_block':256//7,
      'assumptions':['64-bit classic Bulletproof shape; compressed 33-byte secp256k1 points and 32-byte scalars',
      'Aggregate values and MAX_MONEY complements, padded to a power of two',
      'Keep existing non-range-proof overhead; no cryptographic or wire-format compatibility established',
      'No CPU speedup assumed; existing work limits remain unchanged'], 'scenarios':rows}

if __name__=='__main__':
    result=projections(json.loads((ROOT/'report-v2.json').read_text())['validation_cost_samples'])
    (ROOT/'economics.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result,indent=2))
