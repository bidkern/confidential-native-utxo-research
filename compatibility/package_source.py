"""Curated source-only review archive; excludes binaries, wallets and node state."""
from pathlib import Path
import hashlib
import json
import zipfile

ROOT=Path(__file__).resolve().parents[1]
def main():
    paths=[ROOT/'Cargo.toml',ROOT/'Cargo.lock',ROOT/'README.md',ROOT/'integration/lab.py',ROOT/'scripts/cargo.ps1']
    for directory in ('crates','tests','docs'):
        paths += [p for p in (ROOT/directory).rglob('*') if p.is_file() and p.suffix in ('.rs','.toml','.md','.txt','.json')]
    for name in ('README.md','STATUS.md','clean-reproduction.json','SPEC.md','COMPROMISES.md','ACTIVATION.md','REVIEW-PACKAGE.md','REVIEW-REQUEST.md','REPRODUCE.md','PILOT-GATES.md','RESOURCE-LIMITS.md','reserve.cpp','reserve.h','reserve.patch','patch_core.py','build.ps1','reproduce.ps1','package_source.py','demo.py','v2_demo.py','activation_demo.py','hostile_demo.py','report-v2.json','report-activation.json','report-hostile.json'):
        paths.append(ROOT/'compatibility'/name)
    paths += sorted((ROOT/'compatibility/artifacts').glob('clean-report-*.json'))
    paths=sorted(set(paths))
    for p in paths:
        if not p.is_file():raise FileNotFoundError(p)
    output=ROOT/'compatibility/artifacts';output.mkdir(exist_ok=True)
    manifest={p.relative_to(ROOT).as_posix():hashlib.sha256(p.read_bytes()).hexdigest() for p in paths}
    archive=output/'cnu-review-source.zip'
    with zipfile.ZipFile(archive,'w',zipfile.ZIP_DEFLATED) as z:
        for p in paths:z.write(p,'confidential-native-utxo/'+p.relative_to(ROOT).as_posix())
        z.writestr('confidential-native-utxo/SOURCE-SHA256.json',json.dumps(manifest,indent=2)+'\n')
    with zipfile.ZipFile(archive) as z:
        assert z.testzip() is None
        for name,digest in manifest.items():assert hashlib.sha256(z.read('confidential-native-utxo/'+name)).hexdigest()==digest
    (output/'cnu-review-source.sha256').write_text(hashlib.sha256(archive.read_bytes()).hexdigest()+'  '+archive.name+'\n')
    print(json.dumps({'archive':str(archive),'files':len(paths),'bytes':archive.stat().st_size}))
if __name__=='__main__':main()
