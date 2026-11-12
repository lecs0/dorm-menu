#define _GNU_SOURCE
#include <sys/types.h>
#include <sys/resource.h>
#include <sys/prctl.h>
#include <sys/syscall.h>
#include <linux/capability.h>
#include <linux/seccomp.h>
#include <linux/filter.h>
#include <linux/audit.h>
#include <unistd.h>
#include <grp.h>
#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

static void die(const char *message) { perror(message); exit(1); }
static void limit(int resource,rlim_t value) { struct rlimit r={value,value};if(setrlimit(resource,&r))die("setrlimit"); }
static void group(const char *controller,const char *mode) {
    char path[256],pid[32];
    snprintf(path,sizeof path,"/sys/fs/cgroup/%s/dorm-menu-%s-native/tasks",controller,mode);
    int fd=open(path,O_WRONLY);if(fd<0)die("open application cgroup");
    int n=snprintf(pid,sizeof pid,"%ld",(long)getpid());
    if(write(fd,pid,n)!=n)die("join application cgroup");close(fd);
}
static void filter(void) {
    struct sock_filter rules[128];unsigned n=0;
#define ADD(x) do { rules[n++]=(struct sock_filter)x; } while(0)
    ADD(BPF_STMT(BPF_LD|BPF_W|BPF_ABS,4));
    ADD(BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K,AUDIT_ARCH_ARM,1,0));
    ADD(BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_KILL));
    ADD(BPF_STMT(BPF_LD|BPF_W|BPF_ABS,0));
    const unsigned blocked[]={SYS_unshare,SYS_setns,SYS_mount,SYS_umount2,SYS_pivot_root,SYS_chroot,
        SYS_ptrace,SYS_process_vm_readv,SYS_process_vm_writev,SYS_kexec_load,SYS_init_module,
        SYS_finit_module,SYS_delete_module,SYS_reboot,SYS_swapon,SYS_swapoff,SYS_bpf,
        SYS_keyctl,SYS_add_key,SYS_request_key,SYS_perf_event_open};
    for(unsigned i=0;i<sizeof blocked/sizeof *blocked;i++) {
        ADD(BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K,blocked[i],0,1));
        ADD(BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ERRNO|EPERM));
    }
    // clone3 does not exist on this kernel. Permit normal clone fallback, never namespace creation.
    ADD(BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K,435,0,1));
    ADD(BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ERRNO|ENOSYS));
    ADD(BPF_JUMP(BPF_JMP|BPF_JEQ|BPF_K,SYS_clone,0,3));
    ADD(BPF_STMT(BPF_LD|BPF_W|BPF_ABS,16));
    ADD(BPF_JUMP(BPF_JMP|BPF_JSET|BPF_K,0x7e020000U,0,1));
    ADD(BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ERRNO|EPERM));
    ADD(BPF_STMT(BPF_RET|BPF_K,SECCOMP_RET_ALLOW));
    struct sock_fprog program={(unsigned short)n,rules};
    if(prctl(PR_SET_SECCOMP,SECCOMP_MODE_FILTER,&program))die("set targeted syscall restrictions");
}
int main(int argc,char **argv) {
    if(argc<2 || (strcmp(argv[1],"api") && strcmp(argv[1],"tunnel")))return 2;
    if(geteuid()!=0){fputs("Launcher requires root to enter its filesystem jail and drop privileges.\n",stderr);return 1;}
    int api=!strcmp(argv[1],"api");
    int check=argc==3 && !strcmp(argv[2],"--check");
    int pdfcheck=api && argc==3 && !strcmp(argv[2],"--pdf-check");
    if(argc>2 && !check && !pdfcheck)return 2;
    const char *mode=api?"api":"tunnel";
    group("memory",mode);group("cpu",mode);
    limit(RLIMIT_CORE,0);limit(RLIMIT_NOFILE,256);limit(RLIMIT_NPROC,96);limit(RLIMIT_FSIZE,16*1024*1024);
    if(api)limit(RLIMIT_AS,256*1024*1024);
    if(setpriority(PRIO_PROCESS,0,5))die("set service priority");
    for(int cap=0;cap<64;cap++)if(prctl(PR_CAPBSET_DROP,cap,0,0,0) && errno!=EINVAL)die("drop capability bounding set");
    const char *root=api?"/volume1/dorm-menu/native/api-root":"/volume1/dorm-menu/native/tunnel-root";
    if(chroot(root)||chdir("/"))die("enter service filesystem jail");
    uid_t uid=api?10001:10002;
    if(setgroups(0,NULL)||setgid(uid)||setuid(uid))die("drop service UID/GID");
    struct __user_cap_header_struct header={_LINUX_CAPABILITY_VERSION_3,0};
    struct __user_cap_data_struct caps[2]={{0},{0}};
    if(syscall(SYS_capset,&header,caps))die("clear capabilities");
    if(prctl(PR_SET_NO_NEW_PRIVS,1,0,0,0))die("set no-new-privileges");
    filter();
    // Never inherit privileged directory or control descriptors into the application.
    for(int fd=3;fd<65536;fd++)close(fd);
    clearenv();setenv("PATH","/app/bin:/bin",1);setenv("TZ","Europe/Budapest",1);
    if(api) {
        setenv("BIND","127.0.0.1:3000",1);setenv("DATA_DIR","/data",1);setenv("TOKEN_FILE","/config/tokens.json",1);
        setenv("CHECK_INTERVAL_SECONDS","86400",1);setenv("RATE_LIMIT_RPM","30",1);setenv("RUST_LOG","info",1);
        if(pdfcheck)execl("/app/bin/pdftotext","pdftotext","-bbox-layout","-enc","UTF-8","/data/diagnostic.pdf","-",NULL);
        else if(check)execl("/app/dorm-api","dorm-api","clock-check",NULL);
        else execl("/app/dorm-api","dorm-api",NULL);
    } else {
        setenv("SSL_CERT_FILE","/etc/ssl/certs/ca-certificates.crt",1);setenv("GOMEMLIMIT","96MiB",1);
        if(check)execl("/bin/cloudflared","cloudflared","--version",NULL);
        else execl("/bin/cloudflared","cloudflared","tunnel","--config","/config/config.yml","--no-autoupdate","--metrics","127.0.0.1:20241","run",NULL);
    }
    die("execute service");
}
