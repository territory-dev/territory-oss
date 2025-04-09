from pathlib import Path

from flask import send_from_directory
from werkzeug.exceptions import NotFound


FRONTENT_BUILD_DIR = (Path(__file__).parent / '../../../../front/build').resolve()


def frontend_static(path):
    try:
        return send_from_directory(FRONTENT_BUILD_DIR, path)
    except:
        return send_from_directory(FRONTENT_BUILD_DIR, 'index.html')

