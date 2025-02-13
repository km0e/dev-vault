# pkgs = a b c
# noconfirm = t/f
case $noconfirm in
t) noconfirm="" ;;
f) noconfirm="-i" ;;
esac
apk update $noconfirm
pkgs=$(apk version $pkgs | awk -v "pkgs=$pkgs" '
BEGIN {
  split(pkgs, t, " ")
  for (n in t) {
    m[t[n]] = ""
  }
}
/^[^ ]+ policy:$/ {
  pkg = $1
  sub(" policy:", "", pkg)
  delete m[pkg]
}
END {
  u = ""
  for (i in m) {
    u = u " " i
  }
  printf u
}')

cmd="apk add $noconfirm $pkgs"
$cmd
