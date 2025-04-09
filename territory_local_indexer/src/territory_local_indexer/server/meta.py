from flask import current_app

from ..metascan import load_repos, load_builds, load_branches
from .args import encode_branch, decode_branch


def get_repos():
    repos = load_repos(current_app.config['MC'])
    return [
        {
            'id': r,
            'public': True,
            'owner': None,
            'sharedWithUsers': [],
            'name': r,
            'user_name':  'Territory User',
        }
        for r in repos
    ]


def get_repo(repo_id):
    return {}


def get_branches(repo_id):
    mconf = current_app.config['MC']
    return [{'id': encode_branch(b)} for b in load_branches(mconf, repo_id)]


def get_builds(repo_id, branch):
    mconf = current_app.config['MC']
    branch = decode_branch(branch)
    return load_builds(mconf, repo_id, branch)


def get_build(repo_id, branch, build_id):
    return {
        "id": build_id,
        "indexer_version": "tthere",
        "logs": [ ],
        "commit_message": "",
        "code_root": "path:",
        "started": 0,
        "search_index_path": f"search/{repo_id}/{build_id}/all",
        "ended": 0,
        "status": "Ready",
        "ready": True,
        "trie": f"search/{repo_id}/{build_id}/trie",
        "failed": False,
        "code_storage": "firebase+relay",
        "commit": ""
    }
