from dataclasses import dataclass
from pathlib import Path
from random import choices
from string import ascii_lowercase
from shutil import rmtree
from subprocess import Popen, check_call, check_output
from time import sleep
from typing import Literal

from .configure import MetaConf
from .metascan import load_builds, save_builds


@dataclass
class BuildEnv:
    mconf: MetaConf
    repo_id: str
    branch: str
    build_id: str
    code_dir: Path
    index_dir: Path


def index(
    mconf: MetaConf,
    *,
    lang: Literal['c', 'go', 'python'],
    path: Path,
    repo_id: str = None,
    cc_path: Path = None,
    branch: str = None,
):
    if not repo_id:
        repo_id = path.name

    index_dir = mconf.ttdir / 'repos' / repo_id
    index_dir.mkdir(parents=True, exist_ok=True)

    build_id = ''.join(choices(ascii_lowercase, k=10))

    if not branch:
        try:
            branch = _get_branch(path)
        except Exception as e:
            raise RuntimeError('Failed to determine branch.  Use --branch to pass manually.') from e

    env = BuildEnv(
        mconf=mconf,
        repo_id=repo_id,
        branch=branch,
        build_id=build_id,
        code_dir=path,
        index_dir=index_dir)

    try:
        if lang == 'c':
            _index_c(env)
        elif lang == 'go':
            _index_go(env)
        elif lang == 'python':
            _index_python(env)
        else:
            raise ValueError(f'unsupported repo language: {lang}')
    finally:
        rmtree(env.index_dir / 'work', ignore_errors=True)

    builds = load_builds(mconf, repo_id, branch)
    builds.append({
        'id': build_id,

    })
    save_builds(mconf, repo_id, branch, builds)


def _index_c(env: BuildEnv):
    sock_path = env.index_dir / 'indexer.sock'

    indexer_proc = None
    scanner_proc = None
    try:
        indexer_proc = Popen([
            'clangrs',
            '--repo', str(env.code_dir),
            '--repo-id', env.repo_id,
            '--build-id', env.build_id,
            '--storage-mode', 'file',
            '--intermediate-path', str(env.index_dir / 'work'),
            '--db-path', str(env.index_dir / 'db'),
            '--scanner-socket-path', str(sock_path),
            '--outdir', str(env.mconf.ttdir / 'graph'),
            '--log-dir', str(env.index_dir / 'logs'),
            '--compression', 'none',
        ])

        while not sock_path.exists():
            sleep(1)

        args = [
            'cscanner',
            '--repo-path', str(env.code_dir),
            '--sock', str(sock_path),
            '--compile-commands-dir', str(env.code_dir),
        ]
        scanner_proc = Popen(args)

    finally:
        if indexer_proc and indexer_proc.wait() != 0:
            raise RuntimeError('indexer failed')
        if scanner_proc and scanner_proc.wait() != 0:
            raise RuntimeError('scanner failed')


def _index_go(env: BuildEnv):
    uim_path = env.code_dir / '.territory' / 'uim'
    check_call([
        'goscan',
        str(env.code_dir), str(uim_path)
    ])

    check_call([
        'clangrs',
        '--repo', str(env.code_dir),
        '--repo-id', env.repo_id,
        '--build-id', env.build_id,
        '--storage-mode', 'file',
        '--intermediate-path', str(env.index_dir / 'work'),
        '--db-path', str(env.index_dir / 'db'),
        '--scanner-socket-path', str(env.index_dir / 'indexer.sock'),
        '--outdir', str(env.mconf.ttdir / 'graph'),
        '--log-dir', str(env.index_dir / 'logs'),
        '--compression', 'none',
        '--uim-input', str(uim_path),
    ])


def _index_python(env: BuildEnv):
    uim_path = (env.code_dir / '.territory' / 'uim').absolute()
    check_call([
        'python', '-m', 'territory_python_scanner',
        str(env.code_dir.absolute()), str(uim_path)
    ])

    check_call([
        'clangrs',
        '--repo', str(env.code_dir),
        '--repo-id', env.repo_id,
        '--build-id', env.build_id,
        '--storage-mode', 'file',
        '--intermediate-path', str(env.index_dir / 'work'),
        '--db-path', str(env.index_dir / 'db'),
        '--scanner-socket-path', str(env.index_dir / 'indexer.sock'),
        '--outdir', str(env.mconf.ttdir / 'graph'),
        '--log-dir', str(env.index_dir / 'logs'),
        '--compression', 'none',
        '--uim-input', str(uim_path),
    ])


def _get_branch(code_dir: Path):
    return check_output(['git', 'branch', '--show-current'], cwd=code_dir, encoding='utf8').strip()
