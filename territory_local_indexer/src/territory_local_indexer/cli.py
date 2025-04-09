from argparse import ArgumentParser
from pathlib import Path

from . import indexer, server
from .configure import get_metaconf


def serve(mconf, args):
    app = server.mkapp(mconf)
    app.run(debug=True, port=args.port)


def index(mconf, args):
    indexer.index(
        mconf,
        repo_id=args.repo_id,
        lang=args.lang,
        path=args.path.absolute(),
        cc_path=args.cc_path,
        branch=args.branch)


ap = ArgumentParser()
sps = ap.add_subparsers(required=True)

sp = sps.add_parser('serve')
sp.add_argument('--port', type=int)
sp.set_defaults(func=serve)

sp = sps.add_parser('index')
sp.set_defaults(func=index)
sp.add_argument('--cc-path', type=Path)
sp.add_argument('--lang', choices=['c', 'go', 'python'], required=True)
sp.add_argument('--branch')
sp.add_argument('--repo-id')
sp.add_argument('path', type=Path)


def main(argv=None, mconf=None):
    if mconf is None:
        mconf = get_metaconf()
    args = ap.parse_args(argv)
    args.func(mconf, args)
