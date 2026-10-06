# Reproduce the experiment on Windows

Use a fresh extraction of artifacts/cnu-review-source.zip. No wallet seeds, node data directories, private accounts or prebuilt executables are included. SOURCE-SHA256.json identifies every archived source file; verify against the separately supplied archive checksum. These checks identify a package, not its trustworthiness.

Prerequisites: Windows x64, Git, Python 3, Rust with the MSVC toolchain, Visual Studio Build Tools with C++ and bundled CMake/vcpkg. The current machine uses Rust 1.98.1 and Visual Studio 18 2026. Other toolchains are not yet validated. Cargo.lock pins Rust dependencies. Upstream Core is fetched at e8e7e91a1144c378dff4da2e2a562eb0f3f2e1d6; its vcpkg manifest pins a dependency baseline. Network access is needed to obtain source/dependencies.

```powershell
./compatibility/reproduce.ps1
```

For an existing compatible static vcpkg dependency tree:

```powershell
./compatibility/reproduce.ps1 -DependencyRoot 'C:\your\vcpkg\installed'
```

The script refuses an existing Core checkout. It fetches the pinned clean source, runs Rust tests, builds and saves stock Core, patches the source, builds upgraded Core, and runs the three demonstrations. The build accepts an explicit CMake `-Generator` argument. Without an installed dependency directory, vcpkg manifest installation is enabled with GUI/wallet/tests disabled.

Artifacts to inspect: report-v2.json, report-activation.json, report-hostile.json, plus each isolated runs directory. Every report must have success=true. Reports record binary hashes; bit-identical builds across machines are not established. Compare consensus decisions and final invariants rather than random txids or wall-clock timings.

The fixed-height experiment uses -cnuactivationheight=105 only in its own data directories. Never change that value for existing chainstate. Default V2 and hostile tests activate at zero. All upgraded binaries refuse public networks. Node processes started by the scripts are stopped on normal completion/error; an external process kill can require manual cleanup of that run's processes.

The malformed-byte Rust corpus is a deterministic fuzz smoke test. The P2P load cases include a twenty-message burst and sixty seconds of source-limited unique invalid traffic. Neither is equivalent to a coverage-guided fuzz campaign, independent implementation or production load qualification.
