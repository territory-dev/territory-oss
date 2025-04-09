from pathlib import Path
from os import getenv

from yaml import SafeLoader, load, dump

from .configure import MetaConf


def load_repos(mconf: MetaConf) -> list[str]:
    return [ d.name for d in (mconf.ttdir / 'graph' / 'builds').glob('*') ]


def load_branches(mconf: MetaConf, repo: str) -> list[str]:
    return [ d.name.removesuffix('.yml') for d in (mconf.ttdir / 'graph' / 'builds' / repo).glob('*.yml') ]


def load_builds(mconf: MetaConf, repo: str, branch: str) -> list[dict]:
    builds_index_path = _builds_index_path(mconf, repo, branch)
    if not (builds_index_path.exists()):
        return []

    with builds_index_path.open() as f:
        build_list = load(f, SafeLoader)
        build_list.reverse()
        return build_list


def save_builds(mconf: MetaConf, repo: str, branch: str, builds: list[dict]):
    builds_index_path = _builds_index_path(mconf, repo, branch)
    builds_index_path.parent.mkdir(exist_ok=True, parents=True)
    with builds_index_path.open('w') as f:
        dump(builds, f)


def _builds_index_path(mconf: MetaConf, repo: str, branch: str) -> Path:
    builds_dir = mconf.ttdir / 'graph' / 'builds' / repo
    return builds_dir / (_encode_branch(branch)+'.yml')


def _encode_branch(b):
    return b.replace('/', '~')
