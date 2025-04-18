## 测试环境

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
nps 使用 ehang-io/nps仓库 v0.26.10
npipe 使用 tkzcfc/npipe仓库 v1.0.1 （使用TCP连接且未启用tls）
iperf3 使用 ar51an/iperf3-win-builds 仓库 3.18

iperf3服务器 nps服务器 npc客户端 np_client np_server 均在电脑192.168.28.132运行

未压缩未加密
nps添加转发TCP隧道 5202 到  127.0.0.1:5201
nps添加转发UDP隧道 5202 到  127.0.0.1:5201

未压缩未加密
npipe添加转发TCP隧道 5203 到  127.0.0.1:5201 并且隧道入口在服务端 出口在客户端 模拟和nps一样的转发流程
npipe添加转发UDP隧道 5203 到  127.0.0.1:5201 并且隧道入口在服务端 出口在客户端 模拟和nps一样的转发流程


所有测试在本机 192.168.28.204 上进行
```



## TCP转发

```

iperf3 TCP直连
iperf3 -c 192.168.28.132 -p 5201
Connecting to host 192.168.28.132, port 5201
[  5] local 192.168.28.204 port 7227 connected to 192.168.28.132 port 5201
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-1.00   sec  89.2 MBytes   746 Mbits/sec
[  5]   1.00-2.01   sec  94.5 MBytes   784 Mbits/sec
[  5]   2.01-3.01   sec  92.6 MBytes   781 Mbits/sec
[  5]   3.01-4.01   sec  86.9 MBytes   726 Mbits/sec
[  5]   4.01-5.00   sec  86.8 MBytes   734 Mbits/sec
[  5]   5.00-6.00   sec  85.9 MBytes   722 Mbits/sec
[  5]   6.00-7.01   sec  92.4 MBytes   771 Mbits/sec
[  5]   7.01-8.00   sec  95.1 MBytes   801 Mbits/sec
[  5]   8.00-9.01   sec  94.4 MBytes   785 Mbits/sec
[  5]   9.01-10.01  sec  96.5 MBytes   811 Mbits/sec
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-10.01  sec   914 MBytes   766 Mbits/sec                  sender
[  5]   0.00-10.02  sec   914 MBytes   765 Mbits/sec                  receiver


nps TCP转发
iperf3 -c 192.168.28.132 -p 5202
Connecting to host 192.168.28.132, port 5202
[  5] local 192.168.28.204 port 8469 connected to 192.168.28.132 port 5202
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-1.01   sec  49.8 MBytes   413 Mbits/sec
[  5]   1.01-2.01   sec  19.4 MBytes   163 Mbits/sec
[  5]   2.01-3.01   sec  15.4 MBytes   129 Mbits/sec
[  5]   3.01-4.00   sec  17.4 MBytes   146 Mbits/sec
[  5]   4.00-5.01   sec  14.4 MBytes   119 Mbits/sec
[  5]   5.01-6.01   sec  17.0 MBytes   143 Mbits/sec
[  5]   6.01-7.01   sec  16.8 MBytes   141 Mbits/sec
[  5]   7.01-8.01   sec  15.1 MBytes   126 Mbits/sec
[  5]   8.01-9.01   sec  15.0 MBytes   126 Mbits/sec
[  5]   9.01-10.00  sec  15.9 MBytes   134 Mbits/sec
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-10.00  sec   196 MBytes   164 Mbits/sec                  sender
[  5]   0.00-10.01  sec   192 MBytes   161 Mbits/sec                  receiver


npipe TCP转发
iperf3 -c 192.168.28.132 -p 5203
Connecting to host 192.168.28.132, port 5203
[  5] local 192.168.28.204 port 7264 connected to 192.168.28.132 port 5203
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-1.01   sec  91.0 MBytes   757 Mbits/sec
[  5]   1.01-2.01   sec  90.5 MBytes   755 Mbits/sec
[  5]   2.01-3.00   sec  88.1 MBytes   749 Mbits/sec
[  5]   3.00-4.01   sec  90.0 MBytes   752 Mbits/sec
[  5]   4.01-5.01   sec  89.6 MBytes   752 Mbits/sec
[  5]   5.01-6.01   sec  92.9 MBytes   776 Mbits/sec
[  5]   6.01-7.00   sec  91.6 MBytes   774 Mbits/sec
[  5]   7.00-8.01   sec  89.5 MBytes   745 Mbits/sec
[  5]   8.01-9.00   sec  89.8 MBytes   757 Mbits/sec
[  5]   9.00-10.01  sec  88.4 MBytes   739 Mbits/sec
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate
[  5]   0.00-10.01  sec   901 MBytes   756 Mbits/sec                  sender
[  5]   0.00-10.01  sec   901 MBytes   755 Mbits/sec                  receiver
```

## UDP转发

```

iperf3 UDP直连
iperf3 -c 192.168.28.132 -p 5201 -u -b 10G -t 10 -i 1
[  5] local 192.168.28.204 port 52483 connected to 192.168.28.132 port 5201
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-1.00   sec   105 MBytes   880 Mbits/sec  75770
[  5]   1.00-2.01   sec   106 MBytes   880 Mbits/sec  76129
[  5]   2.01-3.01   sec   105 MBytes   880 Mbits/sec  75474
[  5]   3.01-4.01   sec   105 MBytes   880 Mbits/sec  75605
[  5]   4.01-5.00   sec   104 MBytes   880 Mbits/sec  75131
[  5]   5.00-6.01   sec   106 MBytes   880 Mbits/sec  76039
[  5]   6.01-7.01   sec   105 MBytes   880 Mbits/sec  75393
[  5]   7.01-8.01   sec   105 MBytes   879 Mbits/sec  75563
[  5]   8.01-9.00   sec   104 MBytes   880 Mbits/sec  75129
[  5]   9.00-10.01  sec   105 MBytes   880 Mbits/sec  75541
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-10.01  sec  1.02 GBytes   880 Mbits/sec  0.000 ms  0/755774 (0%)  sender
[  5]   0.00-10.01  sec   663 MBytes   556 Mbits/sec  0.024 ms  277781/755420 (37%)  receiver


nps UDP转发
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


npipe UDP转发
iperf3 -c 192.168.28.132 -p 5203 -u -b 10G -t 10 -i 1
Connecting to host 192.168.28.132, port 5203
[  5] local 192.168.28.204 port 61267 connected to 192.168.28.132 port 5203
[ ID] Interval           Transfer     Bitrate         Total Datagrams
[  5]   0.00-1.01   sec   106 MBytes   880 Mbits/sec  76081
[  5]   1.01-2.01   sec   106 MBytes   879 Mbits/sec  75998
[  5]   2.01-3.01   sec   104 MBytes   880 Mbits/sec  75119
[  5]   3.01-4.01   sec   105 MBytes   880 Mbits/sec  75665
[  5]   4.01-5.00   sec   104 MBytes   880 Mbits/sec  75066
[  5]   5.00-6.01   sec   106 MBytes   879 Mbits/sec  76122
[  5]   6.01-7.00   sec   104 MBytes   880 Mbits/sec  74697
[  5]   7.00-8.01   sec   106 MBytes   879 Mbits/sec  76422
[  5]   8.01-9.01   sec   105 MBytes   879 Mbits/sec  75333
[  5]   9.01-10.01  sec   105 MBytes   879 Mbits/sec  75559
- - - - - - - - - - - - - - - - - - - - - - - - -
[ ID] Interval           Transfer     Bitrate         Jitter    Lost/Total Datagrams
[  5]   0.00-10.01  sec  1.03 GBytes   880 Mbits/sec  0.000 ms  0/756062 (0%)  sender
[  5]   0.00-10.03  sec   463 MBytes   388 Mbits/sec  0.008 ms  133324/466947 (29%)  receiver
```

