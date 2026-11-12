#!/usr/bin/env python3
"""Create Docker-save archives from the tested ARMv7 binaries, without a Docker daemon.

The classic Docker image format is intentional: ADM's Docker 18 cannot be assumed
compatible with modern OCI image compression/manifests. No compilation occurs on NAS.
"""
import argparse
import hashlib
import io
import json
from pathlib import Path
import shutil
import tarfile
import tempfile

CREATED = "2026-11-11T00:00:00Z"

def digest(data):
    return hashlib.sha256(data).hexdigest()

def image(root, output, tag, config):
    layer = output.parent / (output.stem + '.layer.tar')
    def normalize(info):
        info.uid = info.gid = 0
        info.uname = info.gname = 'root'
        info.mtime = 0
        return info
    with tarfile.open(layer, 'w', format=tarfile.GNU_FORMAT) as archive:
        for child in sorted(root.iterdir()):
            archive.add(child, arcname=child.name, filter=normalize)
    diff = hashlib.file_digest(layer.open('rb'), 'sha256').hexdigest() if hasattr(hashlib, 'file_digest') else digest(layer.read_bytes())
    configuration = {'architecture':'arm', 'os':'linux', 'created':CREATED,
        'config':config, 'rootfs':{'type':'layers','diff_ids':['sha256:'+diff]},
        'history':[{'created':CREATED,'created_by':'DormMenu verified ARMv7 bundle'}]}
    raw = json.dumps(configuration,sort_keys=True,separators=(',',':')).encode()
    config_name = digest(raw)+'.json'
    layer_name = diff+'/layer.tar'
    manifest = [{'Config':config_name,'RepoTags':[tag],'Layers':[layer_name]}]
    with tarfile.open(output,'w:gz',format=tarfile.GNU_FORMAT) as archive:
        def add_bytes(name, data):
            info=tarfile.TarInfo(name);info.size=len(data);info.mode=0o644;info.mtime=0
            archive.addfile(info,io.BytesIO(data))
        add_bytes(config_name,raw)
        add_bytes('manifest.json',json.dumps(manifest).encode())
        info=tarfile.TarInfo(layer_name);info.size=layer.stat().st_size;info.mode=0o644
        with layer.open('rb') as stream:archive.addfile(info,stream)
    layer.unlink()
    return {'tag':tag,'config_digest':digest(raw),'archive_sha256':digest(output.read_bytes())}

def base(root, busybox):
    for directory in ['bin','etc','data','config','tmp','opt']:(root/directory).mkdir(parents=True,exist_ok=True)
    (root/'tmp').chmod(0o1777)
    shutil.copy2(busybox,root/'bin/busybox')
    for name in ['sh','dirname','wget']:(root/'bin'/name).symlink_to('busybox')
    (root/'etc/passwd').write_text('root:x:0:0:root:/root:/bin/sh\ndorm:x:10001:10001:Dorm API:/data:/bin/sh\ntunnel:x:10002:10002:Tunnel:/tmp:/bin/sh\n')
    (root/'etc/group').write_text('root:x:0:\ndorm:x:10001:\ntunnel:x:10002:\n')

def main():
    p=argparse.ArgumentParser()
    p.add_argument('--bundle',type=Path,default=Path('target/dorm-menu-nas-bundle.tar.gz'))
    p.add_argument('--busybox',type=Path,required=True)
    p.add_argument('--cloudflared',type=Path,required=True)
    p.add_argument('--output',type=Path,default=Path('target/nas-deployment'))
    a=p.parse_args();a.output.mkdir(parents=True,exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='dorm-images-') as tmp:
        work=Path(tmp);api=work/'api';tunnel=work/'tunnel';base(api,a.busybox);base(tunnel,a.busybox)
        app=api/'opt/dorm-menu';app.mkdir(parents=True)
        with tarfile.open(a.bundle) as archive:
            for member in archive.getmembers():
                name=Path(member.name)
                if name.is_absolute() or '..' in name.parts:raise ValueError('Unsafe bundle member')
            archive.extractall(app)
        # Deployment configuration is always supplied as separate, restricted mounts.
        if (app/'config').exists():shutil.rmtree(app/'config')
        shutil.copy2(a.cloudflared,tunnel/'bin/cloudflared')
        (tunnel/'etc/ssl/certs').mkdir(parents=True)
        shutil.copy2('/etc/ssl/certs/ca-certificates.crt',tunnel/'etc/ssl/certs/ca-certificates.crt')
        api_tag='dorm-menu-api:armv7-'+digest((app/'dorm-api').read_bytes())[:12]
        cf_tag='dorm-menu-tunnel:armv7-'+digest(a.cloudflared.read_bytes())[:12]
        api_config={'User':'10001:10001','WorkingDir':'/data','Entrypoint':['/opt/dorm-menu/dorm-api'],
            'Env':['PATH=/opt/dorm-menu/bin:/bin','BIND=0.0.0.0:3000','DATA_DIR=/data','TOKEN_FILE=/config/tokens.json','CHECK_INTERVAL_SECONDS=86400','RATE_LIMIT_RPM=30','RUST_LOG=info']}
        tunnel_config={'User':'10002:10002','WorkingDir':'/tmp','Entrypoint':['/bin/cloudflared'],
            'Cmd':['tunnel','--config','/config/config.yml','--no-autoupdate','--metrics','127.0.0.1:20241','run'],
            'Env':['PATH=/bin','SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt']}
        result={'api':image(api,a.output/'api-image.tar.gz',api_tag,api_config),
                'tunnel':image(tunnel,a.output/'tunnel-image.tar.gz',cf_tag,tunnel_config)}
        (a.output/'images.json').write_text(json.dumps(result,indent=2)+'\n')
        (a.output/'images.env').write_text('API_IMAGE='+api_tag+'\nTUNNEL_IMAGE='+cf_tag+'\n')
        print(json.dumps({k:v['tag'] for k,v in result.items()}))
if __name__=='__main__':main()
