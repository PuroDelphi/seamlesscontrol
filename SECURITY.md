# Security policy

[Español](SECURITY.es.md)

SeamlessControl controls input across computers. Please report vulnerabilities privately so users have time to update before details become public.

## Supported versions

Security fixes target the latest published release and current `main`. Older releases are not maintained. SeamlessControl has not undergone an independent security audit.

## Report a vulnerability

Use [GitHub's private vulnerability reporting](https://github.com/PuroDelphi/seamlesscontrol/security/advisories/new) or email [soporte@asistentesautonomos.com](mailto:soporte@asistentesautonomos.com) with **SeamlessControl security** in the subject. Do not open a public issue or share a working exploit before coordination. Include the affected version or commit, impact, reproduction steps and a minimal proof of concept if available. Remove credentials, pairing secrets, private keys and personal data from logs.

The maintainer will acknowledge and assess reports as capacity allows, coordinate a fix and disclose the issue after affected users can update. No response deadline, bounty or compatibility guarantee is promised. For ordinary bugs and setup questions, see [SUPPORT.md](SUPPORT.md).

## Security boundaries

The user authorizes pairing on both computers by comparing a temporary code. A paired identity is pinned locally. Network control uses authenticated encryption; the receiver accepts one input owner at a time. Firewall access must be limited to the chosen LAN interface, subnet, receiver address and port. The panel previews that rule before requesting authorization. These controls do not replace the security of either host or its local network.
