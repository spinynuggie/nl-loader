# FemboyLose release repo

Releases are built automatically from source hosted at [git.femboy.tw/root/FemboyLose](https://git.femboy.tw/root/FemboyLose).

This repository contains no source code. It only contains the GitHub Actions workflow used to build and publish release artifacts.

If you do not want to use the prebuilt binaries from this repository, compile the project directly from source.

## Download

Grab the latest release from the [Releases](../../releases/latest) page.

Extract the zip and place all three files in the same folder:

| File | Purpose |
|------|---------|
| `neverlose.dll` | Main module, x86 |
| `injector.exe` | DLL injector, x86, requires administrator permissions |
| `neverlose-server.exe` | Local server, x64 |

## Source

All source code lives at [git.femboy.tw/root/FemboyLose](https://git.femboy.tw/root/FemboyLose).

This repository exists only to provide reproducible GitHub Actions builds and release downloads.

## Verifying the build

Every release is built by GitHub Actions directly from the public source repository.

No release binaries are uploaded manually.

Each release includes:

- `femboyloserelease.zip`
- `SHA256SUMS.txt`

The exact source commit used for the build is listed in the release notes and in `SHA256SUMS.txt`.

### Verify the published files

1. Open the Actions run linked in the release notes.
2. In the build log, find the `Clone source from git.femboy.tw` step.
3. Confirm that the printed commit SHA exists on [git.femboy.tw/root/FemboyLose](https://git.femboy.tw/root/FemboyLose).
4. Download `SHA256SUMS.txt` from the release.
5. Extract `femboyloserelease.zip`.
6. Compare the hashes of the extracted files against `SHA256SUMS.txt`.

PowerShell:

```powershell
Get-FileHash neverlose.dll, injector.exe, neverlose-server.exe -Algorithm SHA256
```
