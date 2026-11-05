set -eu
target_port=$1
proxy_port=$2
busybox=$3
{
    printf 'CONNECT 127.0.0.1:%s HTTP/1.1\r\nHost: 127.0.0.1:%s\r\n\r\n' "$target_port" "$target_port"
    i=0
    while [ "$i" -lt 400 ]; do
        printf '%s\n' '{"kind":"flowmux","value":1}'
        i=$((i + 1))
    done
} | "$busybox" nc -n -w 10 127.0.0.1 "$proxy_port"
