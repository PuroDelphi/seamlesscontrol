# Publishing SeamlessControl

[Back to README](../README.md) · [Español](PUBLISHING.es.md)

The Omarchy marketplace reviews an **exact full Git commit SHA**. SeamlessControl builds its agent from that plugin checkout; its installer does not fetch an executable archive from a GitHub Release. It uses `cargo build --locked`, and `agent/Cargo.lock` fixes crate versions and registry checksums. The [mutable-download finding in Omarchy Automations](https://github.com/omacom/omarchy-plugin-marketplace/issues/9276) concerned a separate runtime archive whose expected SHA256 was also downloaded from a mutable release. That specific download path does not exist here. The installer and its privilege requests still require marketplace review.

1. Finish and commit the widget, agent, manifest, documentation and lockfile. Push the commit to `main`.
2. Wait for the **Validate** GitHub Actions workflow to pass. Its **Marketplace submission** job summary automatically shows the exact 40-character commit SHA to submit, along with the lockfile SHA256. There is no SHA to edit by hand for each release.
3. Create the version tag and source-only GitHub Release at that same commit. The **Release source assets** workflow attaches `seamlesscontrol-X.Y.Z-source.tar.gz` and its adjacent `.sha256` file automatically. Wait for that workflow to pass and confirm both assets are visible. A tag makes the version easy to find, but the full commit SHA remains the marketplace review identifier.
4. Submit the full commit SHA and keep the reviewed source at that commit while the marketplace checks it. Any later commit starts a new workflow with a new SHA.

For local verification before submission, run `bash scripts/submission-pin.sh`. It refuses local changes or a local commit that differs from published `main`, validates the Omarchy plugin, builds with the lockfile, and prints the same full commit SHA and lockfile SHA256.

The user-facing `omarchy plugin add` and `omarchy plugin update` commands still fetch current upstream `HEAD`; the marketplace's exact-SHA validation does not make those commands commit-bound. See the [marketplace verification policy](https://github.com/omacom/omarchy-plugin-marketplace/blob/main/VERIFICATION.md).

The release archive is a copy of the tagged source, with a checksum for that downloadable asset. The installer does **not** download this archive; it builds from the Git checkout with `Cargo.lock`. Thus the asset SHA256 and the marketplace Git commit SHA serve different purposes. To create or verify the pair locally, run `bash scripts/package-source-release.sh vX.Y.Z`. The workflow can also be rerun for an existing release through **Run workflow** with its tag; it verifies already uploaded assets before accepting them. Releases contain no prebuilt agent. If one is introduced later, the reviewed checkout must pin its exact SHA256 and the installer must verify it before installation.
