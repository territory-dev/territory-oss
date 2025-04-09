from dataclasses import dataclass
from os import environ
from pathlib import Path


@dataclass
class MetaConf:
    ttdir: Path


def get_metaconf():
    try:
         ttdir = Path(environ['TERRITORY_DIR'])
    except KeyError:
        ttdir = Path.home() / '.territory'

    return MetaConf(ttdir=ttdir)
