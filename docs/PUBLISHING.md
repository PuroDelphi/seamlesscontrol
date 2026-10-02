# Publishing SeamlessControl

[Back to README](../README.md) · [Español](PUBLISHING.es.md)

The Omarchy marketplace validates an **exact full Git commit SHA**. SeamlessControl builds its agent from the same plugin checkout, so there is no separate downloaded runtime archive to pin. The installer uses `cargo build --locked`: `agent/Cargo.lock` fixes crate versions and registry checksums. This is the source-build equivalent of verifying an external release archive before executing it.

1. Finish and commit the widget, agent, manifest, documentation and lockfile. Push the commit to `main`.
2. Wait for the **Validate** GitHub Actions workflow to pass. Its **Marketplace submission** job summary automatically shows the exact 40-character commit SHA to submit, along with the lockfile SHA256. There is no SHA to edit by hand for each release.
3. Submit that commit SHA and keep `main` at that commit while the marketplace checks it. Any later commit starts a new workflow with a new SHA.

For local verification before submission, run `bash scripts/submission-pin.sh`. It refuses local changes or a local commit that differs from published `main`, validates the Omarchy plugin, builds with the lockfile, and prints the same full commit SHA and lockfile SHA256.

The user-facing `omarchy plugin add` and `omarchy plugin update` commands still fetch current upstream `HEAD`; the marketplace's exact-SHA validation does not make those commands commit-bound. See the [marketplace verification policy](https://github.com/omacom/omarchy-plugin-marketplace/blob/main/VERIFICATION.md).

No GitHub release asset is involved in the normal installation. If a prebuilt agent is introduced later, its download must be pinned and verified independently before installation.
