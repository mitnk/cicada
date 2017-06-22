# Cicada Completions

Path completion is available out of box. In addition to this, cicada also
supports simplifed customized completion using YAML file. Put your completion
files under `~/.cicada/completers/`. The completion files look like this:

```
$ ls ~/.cicada/completers/
brew.yaml git.yaml  pip.yaml  vox.yaml

$ cat ~/.cicada/completers/brew.yaml
- doctor
- info
- install
- list
- search
- uninstall
- update
- upgrade

$ brew u<Tab><Tab>
uninstall  update  upgrade
```

Currently, cicada supports maximum 2 level completion:

```
$ cat ~/.cicada/completers/pip.yaml
- install:
    - --force-reinstall
    - -U
    - --upgrade
    - -r
    - --requirement
    - --user
- download
- uninstall
- freeze
- list
- show
- check
- search:
    - --no-cache-dir
    - --timeout
- wheel
- hash
- completion
- help

$ pip ins<Tab>
$ pip install

$ pip install --re<Tab>
$ pip install --requirement
```
