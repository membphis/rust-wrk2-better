
# how to use

```shell
$ wrk2_better -c 2 -d 1 http://127.0.0.1:8080/hello
wrk2 -c 2 -d 1 http://127.0.0.1:8080/hello -R 99999999 -U

Performance Results:
---------------------
Totals      : 37,901
Duration    : 999.94ms
Data read   : 4.19MB
Requests/sec: 37,903
Transfer/sec: 4.19MB

Uncorrected Latency:
---------------------
  50.000%: 60.00us
  75.000%: 61.00us
  90.000%: 66.00us
  99.000%: 123.00us
  99.900%: 214.00us
  99.990%: 419.00us
  99.999%: 804.00us
 100.000%: 804.00us
 ```