# am = pacman/yay/paru
# pkgs = a b c
# noconfirm = t/f
case $noconfirm in
t) noconfirm="--noconfirm" ;;
f) noconfirm="" ;;
esac
$am -Sy $noconfirm
pkgs=$($am -Q $pkgs | awk -v "pkgs=$pkgs" '
BEGIN {
  split(pkgs, t, " ")
  for (n in t) {
    m[t[n]] = ""
  }
}
/^[^ ]+/{
  split($1, a, " ")
  delete m[a[1]]
}
END {
  u = ""
  for (i in m) {
    u = u " " i
  }
  printf u
}')
cmd="$am -S $noconfirm $pkgs"
$cmd
