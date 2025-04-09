from pathlib import Path

from .tt import PyResolver, SharedResolverCache
from territory_client.resolver import NotFound, SyncedResolver, get_base_resolver


class Blob:
    def __init__(self, path: Path):
        self.path = path

    def download_as_bytes(self, *, start: int = 0, end: int = None, raw_download: bool = True) -> bytes:
        with self.path.open('rb') as f:
            f.seek(start)
            if end is not None:
                length = end - start
            else:
                length = -1
            return f.read(length)


class FileStorage:
    def __init__(self, base_path: Path):
        self.base_path = base_path

    def get_blob(self, blob_path: str) -> Blob | None:
        p = self.base_path / blob_path
        if not p.exists():
            return None
        return Blob(p)


def get_resolver(
    resolver_cache: SharedResolverCache,
    repo_id: str,
    build_id: str,
    base_path: Path,
) -> PyResolver:
    storage = FileStorage(base_path)

    root_blob = storage.get_blob(f'builds/{repo_id}/{build_id}')
    if root_blob is None:
        raise NotFound(f'trie blob not found for build {build_id} of repo {repo_id}')
    bytes_ = root_blob.download_as_bytes()
    base_resolver = get_base_resolver(resolver_cache, repo_id, bytes_)
    return SyncedResolver(storage, base_resolver, f'nodes/{repo_id}')
