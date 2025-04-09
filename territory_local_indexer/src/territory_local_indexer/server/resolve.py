from datetime import timedelta as TD
from threading import Lock
import re

from flask import Response, request, make_response, current_app
from werkzeug.exceptions import abort

from territory_client import SharedResolverCache, bytes_to_node
from territory_client.filestorage import FileStorage, get_resolver
from territory_client.resolver import NotFound, BadUrl
from ..configure import MetaConf
from .args import BuildRef, decode_branch


RESOLUTION_MAX_AGE_S = 60 * 60 * 24

master_db_lock = Lock()
resolvers = {}

resolver_cache = SharedResolverCache(1024)


def _get_cached_resolver(mc: MetaConf, build_ref: BuildRef):
    key = (build_ref.repo_id, build_ref.build_id)
    with master_db_lock:
        try:
            resolver = resolvers[key]
        except KeyError:
            resolver = resolvers[key] = get_resolver(
                resolver_cache,
                build_ref.repo_id,
                build_ref.build_id,
                mc.ttdir / 'graph')

    return resolver


def _loc_to_url(res) -> str:
    bytes_ = res['blob_bytes']

    if bytes_:
        l, r = bytes_
        return f'slice:{res["path"]}[{l}:{r}]'
    else:
        return res["path"]


_bad_key_re = re.compile('[./]')
def _check_arg(arg):
    key = request.args[arg]
    if _bad_key_re.search(key):
        raise abort(Response('bad argument', status=400))
    return key
def req_build_ref() -> BuildRef:
    return BuildRef(
        _check_arg('repo_id'),
        decode_branch(request.args['branch']),
        _check_arg('build_id')
    )


def _resolve_url(resolver, url):
    try:
        return resolver.resolve_url(url)
    except NotFound as e:
        abort(Response(f'data not found ({e})', status=404))
    except BadUrl:
        abort(Response(f'bad node URL: {url}', status=404))


def _get_url(resolver, url, **kw):
    try:
        return resolver.get_url(url, **kw)
    except NotFound as e:
        abort(Response(f'data not found ({e})', status=404))
    except BadUrl:
        abort(Response(f'bad node URL: {url}', status=404))


def resolve() -> Response:
    url = request.args['url']
    action = request.args.get('action', 'resolve')

    build_ref = req_build_ref()

    cache_control = f'public, max-age={RESOLUTION_MAX_AGE_S}'

    if request.method == 'OPTIONS':
        response = make_response()
        response.headers['Cache-Control'] = cache_control
        return response

    resolver = _get_cached_resolver(current_app.config['MC'], build_ref)

    match action:
        case 'resolve':
            resolution = _resolve_url(resolver, url)
            return _loc_to_url(resolution)

        case 'relay':
            if request.accept_mimetypes.accept_json:
                (_, bytes_) = _get_url(resolver, url)
                response = bytes_to_node(bytes_)
            else:
                (_, bytes_) = _get_url(resolver, url)
                response = Response(bytes_, headers={
                    "Content-Type": 'application/octet-stream',
                })
            response.headers['Cache-Control'] = cache_control
            return response

        case _:
            return f'bad action: {action}', 400


def search_blob() -> Response:
    if request.method == 'OPTIONS':
        response = make_response()
        return response

    build_ref = req_build_ref()
    mconf = current_app.config['MC']
    storage = FileStorage(mconf.ttdir / 'graph')
    return storage.get_blob(f'search/{build_ref.repo_id}/{build_ref.build_id}/trie').download_as_bytes(), 200
