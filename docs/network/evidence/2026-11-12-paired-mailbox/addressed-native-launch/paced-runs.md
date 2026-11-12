# Source guest admission with a paced host clock

The cold guest boot interface initializes wall time in whole seconds. The signed Cast window in run 8 began 252 ms after that integer seed. Both saved grants passed original signature, nonce, verity and mounted-context checks offline at seed +252 ms and +1 s. They refused at seed +251 ms and after expiry. The guest’s actual evaluation time in run 8 was unmeasured.

Runs 10 and 11 delayed host boot after the original pair prepared its grants. At boot both guest clocks could represent an instant inside their signed windows. No TTL, grant validity or policy check changed. Both roles produced VM start events. Both source SDK calls returned an admitted machine. Both destination SDK calls reported that the named machine was missing. Run 9 had already stopped at a concurrent local volume registry operation before reaching boot.

| Run | Host pacing delay | Setup through SDK returns | Driver CPU through lifecycle | Full lifecycle | Original coordinator exit |
|---|---:|---:|---:|---:|---:|
| 9 | Pacing never reached | 34.507 s | 5.465 s | 134.958 s | 42 |
| 10 | 729.071 ms | 5.943 s | 7.576 s | 106.360 s | 42 |
| 11 | 195.190 ms | 6.181 s | 9.134 s | 106.897 s | 42 |

None of these runs produced guest B payload readback or guest mailbox transfer counters. The host operational audit contained two adjacent JSON records on one line in each run. The VM event inventory counts complete JSON values. Signed plan receipts are separate records. [Recorded bytes, scope, counters and report digests](paced-runs.json) · [Prior activation attempts](guest-activation.md)

[Regression and bounded launch-volume lock correction for run 9](../launch-volume-leases/README.md)
