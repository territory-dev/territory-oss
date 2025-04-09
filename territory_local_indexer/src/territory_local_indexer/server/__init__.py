import logging

from flask import Flask, Response, request, make_response, json, current_app
from flask_cors import CORS

from ..configure import get_metaconf
from .meta import get_repos, get_branches, get_builds, get_repo, get_build
from .resolve import resolve, search_blob
from .static import FRONTENT_BUILD_DIR, frontend_static


def mkapp(mc = None):
    app = Flask(__name__, static_url_path='/static', static_folder=FRONTENT_BUILD_DIR / 'static')
    # app = Flask(__name__)
    CORS(app, origins=['*'])
    app.logger.setLevel(logging.INFO)

    if mc is None:
        app.config['MC'] = get_metaconf()
    else:
        app.config['MC'] = mc

    app.add_url_rule("/api/", view_func=health)
    app.add_url_rule('/api/repos', methods=['GET'], view_func=get_repos)
    app.add_url_rule('/api/repos/<repo_id>', methods=['GET'], view_func=get_repo)
    app.add_url_rule('/api/repos/<repo_id>/branches/', methods=['GET'], view_func=get_branches)
    app.add_url_rule('/api/repos/<repo_id>/branches/<branch>/builds/', methods=['GET'], view_func=get_builds)
    app.add_url_rule('/api/repos/<repo_id>/branches/<branch>/builds/<build_id>', methods=['GET'], view_func=get_build)
    app.add_url_rule('/api/resolve', methods=['GET', 'OPTIONS'], view_func=resolve)
    app.add_url_rule('/api/search-blob', methods=['GET', 'OPTIONS'], view_func=search_blob)
    app.add_url_rule("/", view_func=frontend_static, defaults={'path': 'index.html'})
    app.add_url_rule("/<path:path>", view_func=frontend_static)

    return app


def health():
    current_app.logger.info('healthcheck hit')
    return 'OK', 200

