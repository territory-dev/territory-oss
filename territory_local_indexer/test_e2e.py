import os
from unittest.mock import ANY

from pathlib import Path
from subprocess import run

from territory_client import PyTrieIndex, bytes_to_node

from territory_local_indexer.cli import main
from territory_local_indexer.configure import MetaConf
from territory_local_indexer.server import mkapp


TEST_REPO_ID = 'test_repo'
TEST_REPO_NAME = 'test_repo'
TEST_BRANCH = 'release/18.x'

EXAMPLE_REPO_DIR = (Path(__file__).parent / '../repos/example').resolve()
GO_EXAMPLE_REPO_DIR = (Path(__file__).parent / '../repos/go').resolve()
PYTHON_EXAMPLE_REPO_DIR = (Path(__file__).parent / '../repos/py').resolve()


def test_index_c(tmp_path):
    origin_path = tmp_path / 'origin'
    origin_path.mkdir(parents=True)
    origin_path /=  TEST_REPO_NAME
    init_c_repo(origin_path)

    mconf = MetaConf(ttdir=tmp_path)
    main(['index', '--lang', 'c', '--repo-id', TEST_REPO_NAME, '--branch', TEST_BRANCH, str(origin_path)], mconf)

    assert (tmp_path / 'graph').exists()

    app = mkapp(mc=MetaConf(ttdir = tmp_path))
    tc = app.test_client()

    assert list_repos(tc) == [
        {
            'id': TEST_REPO_NAME,
            'public': True,
            'owner': None,
            'sharedWithUsers': [],
            'name': TEST_REPO_NAME,
            'user_name': 'Territory User',
        },
    ]
    list_branches(tc)
    build, = list_builds(tc)

    root_node = resolve_node(tc, build['id'], 'path:')
    assert root_node == {
        'id': ANY,
        'container': None,
        'kind': 'directory',
        'member_of': None,
        'path': '/',
        'path_id': ANY,
        'references': None,
        'start': {
            'col': 0,
            'line': 0,
            'off': 0,
        },
        'text': [
            {
                'N': 0,
                'T': 'Identifier',
                'h': ANY,
                'ht': None,
                'id': ANY,
                'r': None,
                's': None,
                't': 'dir/\n',
            },
            {
                'N': 1,
                'T': 'Identifier',
                'h': ANY,
                'ht': None,
                'id': ANY,
                'r': None,
                's': None,
                't': 'mod1.c\n',
            },
            {
                'N': 2,
                'T': 'Identifier',
                'h': ANY,
                'ht': None,
                'id': 'tok-12',
                'r': None,
                's': None,
                't': 'shared.h\n',
            },
        ],
    }

    search = get_search(tc, build['id'])
    assert search.search('baz', {}) == [
        {
            'href': ANY,
            'key': 'baz',
            'kind': 'IiSymbol',
            'path': 'mod1.c',
            'positions': [0, 1, 2],
            'score': ANY,
            'type': 'void ()',
        },
    ]


def test_index_go(tmp_path):
    origin_path = tmp_path / 'origin'
    origin_path.mkdir(parents=True)
    origin_path /=  TEST_REPO_NAME
    init_go_repo(origin_path)

    mconf = MetaConf(ttdir=tmp_path)
    main(['index', '--lang', 'go', '--branch', TEST_BRANCH, str(origin_path)], mconf)

    assert (tmp_path / 'graph').exists()

    app = mkapp(mc=MetaConf(ttdir = tmp_path))
    tc = app.test_client()

    assert list_repos(tc) == [
        {
            'id': TEST_REPO_NAME,
            'public': True,
            'owner': None,
            'sharedWithUsers': [],
            'name': TEST_REPO_NAME,
            'user_name': 'Territory User',
        },
    ]
    list_branches(tc)
    build, = list_builds(tc)

    root_node = resolve_node(tc, build['id'], 'path:')
    assert root_node == {
        'id': ANY,
        'container': None,
        'kind': 'directory',
        'member_of': None,
        'path': '/',
        'path_id': ANY,
        'references': None,
        'start': {
            'col': 0,
            'line': 0,
            'off': 0,
        },
        'text': [
            {
                'id': ANY,
                'N': 0,
                'T': 'Identifier',
                'h': ANY,
                'ht': None,
                'r': None,
                's': None,
                't': 'main.go\n',
            },
        ],
    }

    search = get_search(tc, build['id'])
    assert search.search('f', {}) == [
        {
            'href': ANY,
            'key': 'f',
            'kind': 'IiSymbol',
            'path': 'main.go',
            'positions': [0],
            'score': ANY,
            'type': None,
        },
    ]


def test_index_python(tmp_path):
    origin_path = tmp_path / 'origin'
    origin_path.mkdir(parents=True)
    origin_path /=  TEST_REPO_NAME
    init_python_repo(origin_path)

    mconf = MetaConf(ttdir=tmp_path)
    main(['index', '--lang', 'python', '--branch', TEST_BRANCH, str(origin_path)], mconf)

    assert (tmp_path / 'graph').exists()

    app = mkapp(mc=MetaConf(ttdir = tmp_path))
    tc = app.test_client()

    assert list_repos(tc) == [
        {
            'id': TEST_REPO_NAME,
            'public': True,
            'owner': None,
            'sharedWithUsers': [],
            'name': TEST_REPO_NAME,
            'user_name': 'Territory User',
        },
    ]
    list_branches(tc)
    build, = list_builds(tc)

    root_node = resolve_node(tc, build['id'], 'path:')
    assert root_node == {
        'id': ANY,
        'container': None,
        'kind': 'directory',
        'member_of': None,
        'path': '/',
        'path_id': ANY,
        'references': None,
        'start': {
            'col': 0,
            'line': 0,
            'off': 0,
        },
        'text': [
            {
                'id': ANY,
                'N': 0,
                'T': 'Identifier',
                'h': ANY,
                'ht': None,
                'r': None,
                's': None,
                't': 'example.py\n',
            },
        ],
    }

    search = get_search(tc, build['id'])
    assert search.search('f', {}) == [
        {
            'href': ANY,
            'key': 'foo',
            'kind': 'IiSymbol',
            'path': 'example.py',
            'positions': [0],
            'score': ANY,
            'type': None,
        },
    ]


def list_repos(tc):
    resp = tc.get('/api/repos')

    assert resp.status_code == 200
    return resp.json


def list_branches(tc):
    resp = tc.get(f'/api/repos/{TEST_REPO_NAME}/branches/')

    assert resp.status_code == 200
    assert resp.json == [
        { 'id': 'release~18.x' },
    ]


def list_builds(tc):
    resp = tc.get(f'/api/repos/{TEST_REPO_NAME}/branches/release~18.x/builds/')

    assert resp.status_code == 200
    assert resp.json == [
        { 'id': ANY },
    ]
    assert isinstance(resp.json[0]['id'], str)

    return resp.json


def resolve_node(tc, build_id: str, resource: str) -> bytes:
    resp = tc.get('/api/resolve', query_string={
        'repo_id': TEST_REPO_NAME,
        'branch': 'release~18.x',
        'build_id': build_id,
        'url': resource,
        'action': 'relay',
    })

    assert resp.status_code == 200, f'bad status, {resp.text}'

    try:
        return bytes_to_node(resp.data)
    except:
        raise ValueError(f'failed to decode response: {resp.data!r}')


def get_search(tc, build_id: str) -> bytes:
    resp = tc.get('/api/search-blob', query_string={
        'repo_id': TEST_REPO_NAME,
        'branch': 'release~18.x',
        'build_id': build_id,
    })

    assert resp.status_code == 200, f'bad status, {resp.text}'

    try:
        return PyTrieIndex(resp.data)
    except:
        raise ValueError(f'failed to decode response: {resp.data!r}')


def init_c_repo(test_repo_dir):
    run(['cp', '-R', str(EXAMPLE_REPO_DIR), test_repo_dir])
    run(['git', 'init', '-b', TEST_BRANCH, test_repo_dir])
    run(['git', '-C', test_repo_dir, 'add', 'mod1.c', 'dir/mod2.c', 'shared.h', 'Makefile'])
    run(['git', '-C', test_repo_dir, 'commit', '-m', 'initial commit'])


def init_go_repo(test_repo_dir):
    run(['cp', '-R', str(GO_EXAMPLE_REPO_DIR), test_repo_dir])
    run(['git', 'init', '-b', TEST_BRANCH, test_repo_dir])
    run(['git', '-C', test_repo_dir, 'add', test_repo_dir])
    run(['git', '-C', test_repo_dir, 'commit', '-m', 'initial commit'])


def init_python_repo(test_repo_dir):
    run(['cp', '-R', str(PYTHON_EXAMPLE_REPO_DIR), test_repo_dir])
    run(['git', 'init', '-b', TEST_BRANCH, test_repo_dir])
    run(['git', '-C', test_repo_dir, 'add', test_repo_dir])
    run(['git', '-C', test_repo_dir, 'commit', '-m', 'initial commit'])

