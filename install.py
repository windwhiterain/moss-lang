from pathlib import Path
import sys
from time import sleep
import winreg
import ctypes
import os
import platform


def get_home() -> Path:
    name = "Moss"
    system = platform.system()

    if system == "Windows":
        return Path(os.getenv("LOCALAPPDATA")) / name  # type: ignore

    elif system == "Darwin":  # macOS
        return Path.home() / "Library" / "Application Support" / name

    else:  # Linux, BSD, etc.
        return Path.home() / ".local" / "share" / name


def has_symlink_privilege():
    try:
        return ctypes.windll.shell32.IsUserAnAdmin()
    except:
        return False


def run_as_admin():
    params = " ".join(f'"{arg}"' for arg in sys.argv)
    rc = ctypes.windll.shell32.ShellExecuteW(
        None,
        "runas",
        sys.executable,
        params,
        None,
        1,  # 1 = SW_SHOWNORMAL, reuse console if possible
    )
    if rc <= 32:
        raise RuntimeError(f"Elevation failed with code {rc}")
    sys.exit()


def link_to_home(path: Path):
    home = get_home()

    if not path.exists():
        raise FileNotFoundError("build moss-lang first then install!")

    if home.exists() or home.is_symlink():
        unlink_to_home()

    home.parent.mkdir(parents=True, exist_ok=True)

    os.symlink(path, home, target_is_directory=path.is_dir())
    print(f"link: {path} => {home}")


def unlink_to_home():
    home = get_home()
    if not home.exists() and not home.is_symlink():
        print(f"link does not exists: {home}")
        return
    os.rmdir(home)
    print(f"unlink: {home}")


HWND_BROADCAST = 0xFFFF
WM_SETTINGCHANGE = 0x1A


def broadcast_env_change():
    ctypes.windll.user32.SendMessageW(
        HWND_BROADCAST, WM_SETTINGCHANGE, 0, "Environment"
    )


def normalize(p: str) -> str:
    return os.path.normpath(p).rstrip("\\/")


def read_user_path() -> list[str]:
    try:
        key = winreg.OpenKey(
            winreg.HKEY_CURRENT_USER, r"Environment", 0, winreg.KEY_READ
        )
        value, _ = winreg.QueryValueEx(key, "PATH")
    except FileNotFoundError:
        return []

    parts = [normalize(p) for p in value.split(";") if p.strip()]
    return parts


def write_user_path(parts: list[str]):
    value = ";".join(parts)
    key = winreg.OpenKey(
        winreg.HKEY_CURRENT_USER, r"Environment", 0, winreg.KEY_SET_VALUE
    )
    winreg.SetValueEx(key, "PATH", 0, winreg.REG_EXPAND_SZ, value)
    broadcast_env_change()


def add_to_path(path: str):
    path = normalize(path)
    parts = read_user_path()

    if path in parts:
        print(f"already in PATH: {path}")
        return

    parts.append(path)
    write_user_path(parts)
    print(f"add to PATH: {path}")


def remove_from_path(path: str):
    path = normalize(path)
    parts = read_user_path()

    if path not in parts:
        print(f"not found in PATH: {path}")
        return

    parts = [p for p in parts if p != path]
    write_user_path(parts)
    print(f"removed from PATH: {path}")


if __name__ == "__main__":
    if len(sys.argv) > 2:
        print("Usage: python install.py [<if_uninstall>]")
        sys.exit(1)
    try:
        if not has_symlink_privilege():
            run_as_admin()
        path = Path(__file__).parent / "target/debug"
        if len(sys.argv) == 2:
            remove_from_path(str(path))
            unlink_to_home()
            print(f"uninstalled moss at {path}")
        else:
            add_to_path(str(path))
            link_to_home(path)
            print(f"installed moss at {path}")
    except Exception as e:
        print(f"error: {e}")
    sleep(100)
