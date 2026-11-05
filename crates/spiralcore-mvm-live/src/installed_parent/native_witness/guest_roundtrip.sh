set -eu
target_host=$1
target_port=$2
busybox=$0
proxy_port=${3:-1080}
value=$4
case "$value" in 1|2) ;; *) exit 2 ;; esac
scratch=$("$busybox" mktemp -d)
trap '"$busybox" rm -r "$scratch"' EXIT
i=0
while [ "$i" -lt 400 ]; do
    printf '{"kind":"flowmux","value":%s}\n' "$value"
    i=$((i + 1))
done > "$scratch/expected"
{
    printf 'CONNECT %s:%s HTTP/1.1\r\nHost: %s:%s\r\n\r\n' "$target_host" "$target_port" "$target_host" "$target_port"
    "$busybox" cat "$scratch/expected"
} | "$busybox" nc -n -w 12 127.0.0.1 "$proxy_port" > "$scratch/reply"
{
    cr=$(printf '\r')
    IFS= read -r status
    [ "$status" = "HTTP/1.1 200 Connection established${cr}" ]
    IFS= read -r end_headers
    [ "$end_headers" = "$cr" ]
    "$busybox" cat > "$scratch/recovered"
} < "$scratch/reply"
"$busybox" cmp "$scratch/expected" "$scratch/recovered"
printf '%s\n' 'spiralcore-guest-readback-exact'
