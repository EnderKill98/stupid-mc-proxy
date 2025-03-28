# stupid-mc-proxy

This program as initially started to allow surviving a giant farm that seemingly DOSed some poor ISPs or routers with too many TCP packets.

It can also be used as a general MC proxy which can sometimes elimite lags due to bad routes or other congestion.

Currently, for each connection it forwards traffic roughly every 25ms to attempt to reduce TCP packet count.

Do not use this for PvPing when you have an otherwise good ping (as this will effectively introduce >25ms of random extra lag). I'm not responsible for increased T-Fails with this!

## How to run

- Build yourself or get the executable from the [Releases Section](https://github.com/EnderKill98/stupid-mc-proxy/releases)
- Run it e.g. like this: `$ stupid-mc-proxy connect.2b2t.org`
- Connect to this proxy (e.g. to "localhost" when running on your PC, or the IP of your VPS)

## Troubleshooting

If connecting doesn't seem possible:
 - Try adding `--bind 0.0.0.0:25565` to your prompt, as your OS might not do automatic dual-stack when binding to IPv6 and you're trying only IPv4 💀
 - Ensure that your firewall(s) are not blocking it
