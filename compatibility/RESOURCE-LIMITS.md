# Experimental resource limits and costs

The following are implemented research limits, not production recommendations:

| Limit | Value |
|---|---:|
| Extension native inputs or outputs | 256 each |
| Encoded payload | 250,000 bytes |
| Metadata | 500,000 bytes |
| Transaction work | 128 units |
| Block work | 256 units |
| Successful proof cache | 1,024 complete statements |

Work is charged per extension transaction: one for the exact-zero excess proof, two per confidential output, two per confidential input, and one per non-reserve transparent input for its inner signature. Normal Bitcoin weight and script/sigop rules still apply. Shared constants drive transaction validation, block validation and mining-template selection. Cache hits do not reduce charged work.

A one-confidential-input/two-confidential-output payment costs seven units; at most 36 fit under the block work cap even if they fit within weight. The cap is deliberately conservative, but this throughput is a serious product limitation. It must not be advertised as a scalable confidential Bitcoin payment system without changing the proof system or justifying different limits.

The near-full-block test fills remaining weight with an ordinary unspendable data output. It therefore tests physical block handling plus the permitted proof workload, not four million weight units entirely occupied by confidential proofs. Five 24-output deposits cost 250 units; six cost 300 and must fail. The mining template must select no more than five from six available such deposits.

Tests also cover twelve separate claims initially selecting one backing fragment, different withdrawal fee rates, a twenty-message invalid-proof burst, and sixty seconds of newly generated invalid-proof traffic. The latter is one source-limited peer with ping round trips, not adversarial network saturation. It proves bounded handling of that workload, not immunity to DoS. Exit tests use one wallet controlling separate claims and do not establish multi-user fairness.

The 24-output wire sample is about 209 KB / 53,050 vbytes. At hypothetical fee rates of 1 and 5 sats/vbyte it would cost 53,050 and 265,250 sats respectively. The lab explicitly lowers relay/mining fee floors; its 1,000-sat example fees are not evidence of normal fee-market viability.

Before production: benchmark cold-cache CPU and peak memory on representative hardware; test bandwidth/connection saturation and cache churn; fuzz parser/FFI boundaries; derive a defensible resource unit schedule; evaluate proof-size reductions through reviewed cryptography; and analyze the resulting throughput and fee tradeoffs. Faster repeat verification does not satisfy these gates.
