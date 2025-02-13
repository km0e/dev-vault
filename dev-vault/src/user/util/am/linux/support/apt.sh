# pkgs = a b c
# noconfirm = t/f
case $noconfirm in
t) noconfirm="-y" ;;
f) noconfirm="" ;;
esac
apt-get update $noconfirm
pkgs=$(apt-cache policy $pkgs | awk -v "pkgs=$pkgs" '
BEGIN {
  split(pkgs, t, " ")
  for (n in t) {
    m[t[n]] = ""
  }
}
/^[^ ]+:$/ {
  pkg = $1
  sub(":", "", pkg)
  delete m[pkg] 
}
END {
  u = ""
  for (i in m) {
    u = u " " i
  }
  printf u
}')

cmd="apt-get install $noconfirm $pkgs"
$cmd
