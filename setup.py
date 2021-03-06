# ARM Pack Manager
# Copyright (c) 2017 ARM Limited
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

from os import getenv
from subprocess import check_output, CalledProcessError
from setuptools import setup
from distutils.version import StrictVersion
from os.path import join, dirname


def build_native(spec):
    build = spec.add_external_build(
        cmd=['cargo', 'build', '--release', '--lib', '--features=cffi'],
        path=join(dirname(__file__), 'rust')
    )

    spec.add_cffi_module(
        module_path='cmsis_pack_manager._native',
        dylib=lambda: build.find_dylib('cmsis_cffi',
                                       in_path='target/release/deps'),
        header_filename=lambda: build.find_header('cmsis.h', in_path='target')
    )


try:
    # Use exact tag, when we're on a tag.
    current_commit = check_output(["git", "log", "-n1", "--pretty=%h"]).strip()
    cmd = ["git", "describe", "--exact-match", "--tags", current_commit]
    exact_match = check_output(cmd).strip()
    version = exact_match.strip("v")
except CalledProcessError:
    version = "0.1.1"

setup(
    name="cmsis-pack-manager",
    version=version,
    packages=["cmsis_pack_manager"],
    zip_safe=False,
    platforms='any',
    setup_requires=[
        'milksnake>=0.1.2',
        'pytest-runner'],
    install_requires=[
        'appdirs>=1.4',
        'milksnake>=0.1.2',
        'pyyaml>=3.12'],
    tests_require=[
        'hypothesis',
        'jinja2',
        'mock',
        'pytest'],
    entry_points={
        'console_scripts': [
            'pack-manager=cmsis_pack_manager.pack_manager:main'
        ]
    },
    milksnake_tasks=[build_native],
    test_suite="tests"
)
