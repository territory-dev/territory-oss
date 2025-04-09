from threading import Lock
from typing import Protocol

from .tt import PyResolver, SharedResolverCache, PyNeedData


_global_resolver_lock = Lock()


class NotFound(Exception):
    pass


class BadUrl(Exception):
    pass


class Blob(Protocol):
    def download_as_bytes(self, *, start: int, end: int, raw_download: bool) -> bytes:  ...


class BlobStorage(Protocol):
    def get_blob(self, blob_path: str) -> Blob:  ...



class SyncedResolver:
    def __init__(self, storage: BlobStorage, base_resolver: PyResolver, code_path: str):
        self.base_resolver = base_resolver
        self.storage = storage
        self.code_path = code_path

        self.master_lock = Lock()  # for accessing the locks dict below
        self.blob_gets = {}

    def resolve_url(self, url):
        while True:
            with _global_resolver_lock:
                try:
                    resolution = self.base_resolver.resolve_url(url)
                except KeyError:
                    raise NotFound(url)
                except ValueError:
                    raise BadUrl(url)

            if isinstance(resolution, PyNeedData):
                self.get_blob_synced(resolution, False)
                continue

            else:
                break

        return resolution

    def get_url(self, url, no_transform=False):
        resolution = self.resolve_url(url)
        return self._get_blob(*self._unpack_location(resolution, no_transform))

    def get_blob_synced(self, resolution, no_transform: bool):
        concrete_location = resolution.location()

        args = self._unpack_location(concrete_location, no_transform)

        blob_get = None
        will_fetch = False
        with self.master_lock:
            try:
                blob_get = self.blob_gets[args]
            except KeyError:
                will_fetch = True
                blob_get = self.blob_gets[args] = Lock()

        with blob_get:
            if will_fetch:
                try:
                    _, resolver_data = self._get_blob(*args)
                    resolution.got_data(resolver_data)
                finally:
                    del self.blob_gets[args]

    def _get_blob(self, blob_path, l, r, no_transform: bool):
        blob = self.storage.get_blob(blob_path)
        if not blob:
            raise NotFound(f'blob for not found {blob_path}')

        bytes_ = blob.download_as_bytes(start=l, end=r, raw_download=no_transform)
        return (blob, bytes_)

    def _unpack_location(self, concrete_location, no_transform):
        blob_path = f'{self.code_path}/{concrete_location["path"]}'

        bytes_ = concrete_location["blob_bytes"]
        if bytes_:
            l, r = bytes_
        else:
            l, r = None, None

        return (blob_path, l, r, no_transform)


def get_base_resolver(resolver_cache: SharedResolverCache, repo_id, build_bytes) -> PyResolver:
    with _global_resolver_lock:
        base_resolver = resolver_cache.get_trie_resolver(repo_id, build_bytes)
    return base_resolver
