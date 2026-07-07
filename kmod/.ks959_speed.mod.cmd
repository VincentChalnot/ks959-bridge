savedcmd_ks959_speed.mod := printf '%s\n'   ks959_speed.o | awk '!x[$$0]++ { print("./"$$0) }' > ks959_speed.mod
