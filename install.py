#!/usr/bin/env python

import os
import shutil
from os import path
from subprocess import run

def main():
    check_root_permission()
    
    src_dir =  path.dirname(path.abspath(__file__))
    binary_src = f'{src_dir}/target/release/umberwm'
    binary_dest = '/usr/bin/umberwm'
    start_script_src = f'{src_dir}/umberwm-start'
    start_script_dest = '/usr/bin/umberwm-start'
    desktop_src = f'{src_dir}/umberwm.desktop'
    desktop_dest = '/usr/share/xsessions/umberwm.desktop'

    # Bail if binary does not exist
    if not path.exists(binary_src):
        exit("Error: binary not found. Run 'cargo build --release' and try again.")
    # Symlink binary to /usr/bin
    symlink(binary_src, binary_dest)
    # Symlink start script to /usr/bin
    symlink(start_script_src, start_script_dest)
    # Copy desktop file to /usr/share/xsessions (it cannot be symlinked)
    print(f"Coppying '{desktop_src}' -> '{desktop_dest}'.")
    shutil.copyfile(desktop_src, desktop_dest)


def symlink(src, dest):
    if path.exists(dest):
        print(f"Removing old '{src}'.")
        os.remove(dest)
    print(f"Symlinking '{src}' -> '{dest}'.")
    os.symlink(src, dest)


def check_root_permission():
    if os.geteuid() != 0:
        exit('Error: Permission denied. Try again with sudo.')


if __name__ == '__main__':
    main()

