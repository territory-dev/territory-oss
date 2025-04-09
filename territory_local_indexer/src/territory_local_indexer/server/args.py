from collections import namedtuple
import re

from flask import Response, request
from werkzeug.exceptions import abort


BuildRef = namedtuple('BuildRef', ['repo_id', 'branch', 'build_id'])


def encode_branch(b):
    return b.replace('/', '~')


def decode_branch(b):
    return b.replace('~', '/')


_bad_key_re = re.compile('[./]')
def check_arg(arg):
    key = request.args[arg]
    if _bad_key_re.search(key):
        raise abort(Response('bad argument', status=400))
    return key


def req_build_ref() -> BuildRef:
    return BuildRef(
        check_arg('repo_id'),
        decode_branch(request.args['branch']),
        check_arg('build_id')
    )
