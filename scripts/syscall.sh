pid=$(ps -ef | grep server_proxy | grep -v grep | awk '{print $2}')
echo 'pid:' $pid
sudo dtrace -n 'syscall:::entry { @num[probefunc] = count(); } tick-20s { exit(0); }' -p $pid
