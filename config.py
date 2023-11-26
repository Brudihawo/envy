"""Configuration utilities for envy"""
import os
import json
from dataclasses import dataclass 

DEFAULT_ENVY_CFG_LOCATION = "~"
DEFAULT_ENVY_CFG_NAME = ".envy/config.json"


@dataclass
class Config:
    DEFAULT = { 
        "root_dir": os.path.join(os.path.expanduser("~"), "notes"),
        "papers_dirname": "papers",
        "daily_dirname": "daily",
        "serve_path": os.path.join(os.path.expanduser("~"), ".envy", "web")
    }

    root_dir: str
    papers_dirname: str
    daily_dirname: str
    serve_path: str

    @classmethod
    def default(cls) -> 'Config':
        return cls(**Config.DEFAULT)

    @classmethod
    def from_cfg(cls, cfg: dict[str, str]) -> 'Config':
        for k, v in Config.DEFAULT.items():
            if k not in cfg:
                cfg[k] = v
        return cls(**cfg)


def get_config(fpath: str | None = None) -> Config:
    """Get a valid configuration for envy.

    This accesses the default envy config file location or a provided config file
    if it is present, and defaults to `DEFAULT_ENVY_CFG` for missing entries.

    Args:
        fpath (str | None): file path to config file. If None, use default path
    """
    if fpath is not None:
        cfg_file_path = fpath
    else:
        cfg_file_path = os.path.join(
            os.path.expanduser(DEFAULT_ENVY_CFG_LOCATION), DEFAULT_ENVY_CFG_NAME
        )

    if os.path.exists(cfg_file_path):
        with open(cfg_file_path, "r") as f:
            cfg = json.load(f)
        return Config.from_cfg(cfg)
    else:
        return Config.default()

