## Testing environment

```
192.168.28.132:
Windows
AMD Ryzen 7 5800H with Radeon Graphics            3.20 GHz
RAM  16.0 GB


192.168.28.204:
Windows
12th Gen Intel(R) Core(TM) i5-12400   2.50 GHz
RAM	32.0 GB
```



```

nps uses ehang io/nps repository v0.26.10
npipe uses tkzcfc/npipe repository v1.0.1 (using TCP connection and tls not enabled)
iperf3 uses ar51an/iperf3 win builds repository 3.18
iperf3 server nps server NPC client np_cient np_server is running on computer 192.168.28.132


nps adds forwarding TCP tunnel 5202 to 127.0.0.1:5201 (Uncompressed and unencrypted)
nps adds forwarding UDP tunnel 5202 to 127.0.0.1:5201 (Uncompressed and unencrypted)


npipe adds forwarding TCP tunnel 5203 to 127.0.0.1:5201, with the tunnel entrance at the server exit and the client simulating the same forwarding process as nps (Uncompressed and unencrypted)
npipe adds forwarding UDP tunnel 5203 to 127.0.0.1:5201, and the tunnel entrance is at the server exit, simulating the same forwarding process as nps on the client side (Uncompressed and unencrypted)

All tests were conducted on the local machine 192.168.28.204
```



## TCP forwarding test

```

iperf3
iperf3 -c 192.168.28.132 -p 5201
Connecting to host 192.168.28.132, port 5201
[  5] local 192.168.28.204 port 10185 connected to 192.168.28.132 port 5201
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-1.00   sec  95.4 MBytes   799 Mbits/sec
[  5]   1.00-2.01   sec  98.4 MBytes   820 Mbits/sec
[  5]   2.01-3.01   sec  98.6 MBytes   823 Mbits/sec
[  5]   3.01-4.01   sec  97.8 MBytes   822 Mbits/sec
[  5]   4.01-5.00   sec  97.9 MBytes   830 Mbits/sec
[  5]   5.00-6.01   sec   100 MBytes   837 Mbits/sec
[  5]   6.01-7.01   sec  98.1 MBytes   821 Mbits/sec
[  5]   7.01-8.01   sec  98.8 MBytes   825 Mbits/sec
[  5]   8.01-9.00   sec  96.1 MBytes   814 Mbits/sec
[  5]   9.00-10.01  sec  96.0 MBytes   803 Mbits/sec
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-10.01  sec   977 MBytes   819 Mbits/sec                  sender
[  5]   0.00-10.01  sec   977 MBytes   819 Mbits/sec                  receiver


nps
iperf3 -c 192.168.28.132 -p 5202
Connecting to host 192.168.28.132, port 5202
[  5] local 192.168.28.204 port 10463 connected to 192.168.28.132 port 5202
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-1.00   sec  49.0 MBytes   409 Mbits/sec
[  5]   1.00-2.00   sec  15.8 MBytes   132 Mbits/sec
[  5]   2.00-3.01   sec  15.9 MBytes   132 Mbits/sec
[  5]   3.01-4.01   sec  16.1 MBytes   135 Mbits/sec
[  5]   4.01-5.01   sec  16.1 MBytes   136 Mbits/sec
[  5]   5.01-6.01   sec  15.9 MBytes   133 Mbits/sec
[  5]   6.01-7.01   sec  15.9 MBytes   133 Mbits/sec
[  5]   7.01-8.00   sec  16.6 MBytes   140 Mbits/sec
[  5]   8.00-9.00   sec  15.5 MBytes   130 Mbits/sec
[  5]   9.00-10.00  sec  16.1 MBytes   135 Mbits/sec
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-10.00  sec   193 MBytes   162 Mbits/sec                  sender
[  5]   0.00-10.01  sec   190 MBytes   160 Mbits/sec                  receiver


npipe
iperf3 -c 192.168.28.132 -p 5203
Connecting to host 192.168.28.132, port 5203
[  5] local 192.168.28.204 port 10485 connected to 192.168.28.132 port 5203
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-1.01   sec  48.0 MBytes   399 Mbits/sec
[  5]   1.01-2.01   sec  46.0 MBytes   384 Mbits/sec
[  5]   2.01-3.01   sec  87.4 MBytes   733 Mbits/sec
[  5]   3.01-4.00   sec  88.6 MBytes   752 Mbits/sec
[  5]   4.00-5.01   sec  87.1 MBytes   728 Mbits/sec
[  5]   5.01-6.01   sec  90.8 MBytes   758 Mbits/sec
[  5]   6.01-7.01   sec  90.9 MBytes   765 Mbits/sec
[  5]   7.01-8.01   sec  88.5 MBytes   743 Mbits/sec
[  5]   8.01-9.00   sec  88.2 MBytes   742 Mbits/sec
[  5]   9.00-10.01  sec  90.9 MBytes   761 Mbits/sec
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-10.01  sec   806 MBytes   676 Mbits/sec                  sender
[  5]   0.00-10.01  sec   806 MBytes   675 Mbits/sec                  receiver
```

## UDP forwarding test

```

iperf3
iperf3 -c 192.168.28.132 -p 5201 -u -b 10G -t 10 -i 1
Connecting to host 192.168.28.132, port 5201
[  5] local 192.168.28.204 port 49579 connected to 192.168.28.132 port 5201
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-1.01   sec   106 MBytes   879 Mbits/sec  76269
[  5]   1.01-2.00   sec   104 MBytes   878 Mbits/sec  74954
[  5]   2.00-3.01   sec   105 MBytes   878 Mbits/sec  75922
[  5]   3.01-4.00   sec   104 MBytes   878 Mbits/sec  74640
[  5]   4.00-5.00   sec   105 MBytes   879 Mbits/sec  75441
[  5]   5.00-6.00   sec   105 MBytes   878 Mbits/sec  75402
[  5]   6.00-7.00   sec   105 MBytes   879 Mbits/sec  75613
[  5]   7.00-8.00   sec   105 MBytes   878 Mbits/sec  75422
[  5]   8.00-9.00   sec   104 MBytes   878 Mbits/sec  75138
[  5]   9.00-10.00  sec   105 MBytes   878 Mbits/sec  75523
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-10.00  sec  1.02 GBytes   878 Mbits/sec  0.000 ms  0/754324 (0%)  sender
[  5]   0.00-10.00  sec   692 MBytes   580 Mbits/sec  0.011 ms  256178/754322 (34%)  receiver

nps
iperf3 -c 192.168.28.132 -p 5202 -u -b 10G -t 10 -i 1
Connecting to host 192.168.28.132, port 5202
[  5] local 192.168.28.204 port 63592 connected to 192.168.28.132 port 5202
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-1.01   sec   106 MBytes   881 Mbits/sec  76362
[  5]   1.01-2.01   sec   105 MBytes   881 Mbits/sec  75601
[  5]   2.01-3.00   sec   104 MBytes   880 Mbits/sec  75138
[  5]   3.00-4.01   sec   105 MBytes   880 Mbits/sec  75583
[  5]   4.01-5.00   sec   105 MBytes   879 Mbits/sec  75478
[  5]   5.00-6.01   sec   106 MBytes   879 Mbits/sec  75980
[  5]   6.01-7.01   sec   105 MBytes   879 Mbits/sec  75368
[  5]   7.01-8.01   sec   105 MBytes   879 Mbits/sec  75493
[  5]   8.01-9.00   sec   104 MBytes   880 Mbits/sec  74863
[  5]   9.00-10.00  sec   105 MBytes   879 Mbits/sec  75619
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-10.00  sec  1.02 GBytes   880 Mbits/sec  0.000 ms  0/755485 (0%)  sender
[  5]   0.00-10.01  sec  59.4 MBytes  49.8 Mbits/sec  0.414 ms  710963/753762 (94%)  receiver


npipe
iperf3 -c 192.168.28.132 -p 5203 -u -b 10G -t 10 -i 1
Connecting to host 192.168.28.132, port 5203
[  5] local 192.168.28.204 port 52439 connected to 192.168.28.132 port 5203
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-1.00   sec   105 MBytes   879 Mbits/sec  75812
[  5]   1.00-2.00   sec   104 MBytes   879 Mbits/sec  75145
[  5]   2.00-3.01   sec   106 MBytes   878 Mbits/sec  76264
[  5]   3.01-4.01   sec   104 MBytes   878 Mbits/sec  75257
[  5]   4.01-5.01   sec   105 MBytes   878 Mbits/sec  75390
[  5]   5.01-6.01   sec   104 MBytes   879 Mbits/sec  75237
[  5]   6.01-7.01   sec   105 MBytes   878 Mbits/sec  75749
[  5]   7.01-8.01   sec   105 MBytes   878 Mbits/sec  75431
[  5]   8.01-9.01   sec   105 MBytes   878 Mbits/sec  75261
[  5]   9.01-10.01  sec   105 MBytes   878 Mbits/sec  75575
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-10.01  sec  1.02 GBytes   878 Mbits/sec  0.000 ms  0/755121 (0%)  sender
[  5]   0.00-10.01  sec   420 MBytes   352 Mbits/sec  0.034 ms  452499/754984 (60%)  receiver
```

