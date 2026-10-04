"""Download an isolated S2 inference runtime/model. Never uploads user audio."""
import hashlib
import json
import os
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed
import time
import zipfile
import requests

ROOT = Path(__file__).resolve().parent / "s2_local_test"
ROOT.mkdir(exist_ok=True)
os.environ["HF_HUB_DISABLE_XET"] = "1"

def direct_session():
    session = requests.Session()
    session.trust_env = False
    return session

def ranged_download(url, path, size):
    if path.exists() and path.stat().st_size == size:
        return
    parts = path.parent / (path.name + ".parts-1mb")
    parts.mkdir(parents=True, exist_ok=True)
    block = 1 << 20
    def fetch(index):
        start = index * block
        end = min(size, start + block) - 1
        part = parts / str(index)
        if part.exists() and part.stat().st_size == end - start + 1:
            return
        for attempt in range(5):
            try:
                with direct_session().get(url, headers={"Range": f"bytes={start}-{end}"},
                                          stream=True, timeout=(20, 30)) as response:
                    response.raise_for_status()
                    if response.status_code != 206 or not response.headers.get("Content-Range", "").startswith(f"bytes {start}-{end}/"):
                        raise RuntimeError("Server ignored requested byte range")
                    with part.open("wb") as output:
                        for chunk in response.iter_content(65536):
                            output.write(chunk)
                if part.stat().st_size != end - start + 1:
                    raise RuntimeError("Incomplete download block")
                return
            except Exception:
                if attempt == 4:
                    raise
                time.sleep(1 + attempt)
    count = (size + block - 1) // block
    with ThreadPoolExecutor(max_workers=12) as pool:
        jobs = [pool.submit(fetch, i) for i in range(count)]
        for finished, job in enumerate(as_completed(jobs), 1):
            job.result()
            if finished % 12 == 0 or finished == count:
                print(f"{path.name}: {finished}/{count} blocks ({finished/count:.0%})", flush=True)
    temporary = path.with_suffix(path.suffix + ".assembling")
    with temporary.open("wb") as output:
        for index in range(count):
            with (parts / str(index)).open("rb") as source:
                for chunk in iter(lambda: source.read(1 << 20), b""):
                    output.write(chunk)
    temporary.replace(path)

def runtime():
    session = direct_session()
    release = session.get("https://api.github.com/repos/0xShug0/audio.cpp/releases/tags/v0.9.0", timeout=30)
    release.raise_for_status()
    asset = next(a for a in release.json()["assets"] if a["name"] == "audio-v0.9.0-bin-windows-x64-vulkan.zip")
    archive = ROOT / asset["name"]
    ranged_download(asset["browser_download_url"], archive, asset["size"])
    if archive.stat().st_size != asset["size"]:
        raise RuntimeError("Runtime archive size mismatch")
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    if asset.get("digest") and asset["digest"] != "sha256:" + digest:
        raise RuntimeError("Runtime archive checksum mismatch")
    destination = ROOT / "runtime"
    destination.mkdir(exist_ok=True)
    with zipfile.ZipFile(archive) as bundle:
        for info in bundle.infolist():
            if not (destination / info.filename).resolve().is_relative_to(destination.resolve()):
                raise RuntimeError("Unsafe archive path")
        bundle.extractall(destination)
    print("Verified runtime downloaded and extracted", flush=True)
    return dict(release=release.json()["tag_name"], asset=asset["name"], sha256=digest)

def model():
    filename = "Fish-Audio-S2-Pro-GGUF/fish-audio-s2-pro-q8_0.gguf"
    metadata = direct_session().get("https://huggingface.co/api/models/audio-cpp/audio.cpp-gguf?blobs=true", timeout=60)
    metadata.raise_for_status()
    revision = metadata.json()["sha"]
    entry = next(e for e in metadata.json()["siblings"] if e["rfilename"] == filename)
    path = ROOT / "models" / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    ranged_download(f"https://huggingface.co/audio-cpp/audio.cpp-gguf/resolve/{revision}/{filename}",
                    path, entry["lfs"]["size"])
    digest = hashlib.file_digest(path.open("rb"), "sha256").hexdigest() if hasattr(hashlib, "file_digest") else None
    if digest is None:
        hasher = hashlib.sha256()
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(8 << 20), b""):
                hasher.update(chunk)
        digest = hasher.hexdigest()
    if entry.get("lfs", {}).get("sha256") != digest:
        raise RuntimeError("Model checksum mismatch")
    print("Verified S2 Pro model downloaded", flush=True)
    return dict(repository="audio-cpp/audio.cpp-gguf", revision=revision, path=str(path), sha256=digest)

if __name__ == "__main__":
    with ThreadPoolExecutor(max_workers=2) as pool:
        binary = pool.submit(runtime)
        weights = pool.submit(model)
        result = dict(runtime=binary.result(), model=weights.result())
    (ROOT / "download_metadata.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
    print("Isolated S2 installation files ready", flush=True)
