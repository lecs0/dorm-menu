#define _GNU_SOURCE
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/file.h>
#include <sys/wait.h>
#include <sys/select.h>
#include <signal.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <errno.h>

static volatile sig_atomic_t stopping;
static void stop(int sig){(void)sig;stopping=1;}
static const char *base="/volume1/dorm-menu/native";
static int valid(const char *mode){return !strcmp(mode,"api") || !strcmp(mode,"tunnel");}
static int same_process(pid_t pid) {
    char path[64];struct stat a,b;
    snprintf(path,sizeof path,"/proc/%ld/exe",(long)pid);
    return !stat(path,&a) && !stat("/volume1/dorm-menu/native/bin/dorm-supervisor",&b) && a.st_dev==b.st_dev && a.st_ino==b.st_ino;
}
static int logfd(const char *mode) {
    char path[256],old[256];struct stat st;
    snprintf(path,sizeof path,"%s/logs/%s.log",base,mode);
    snprintf(old,sizeof old,"%s/logs/%s.previous.log",base,mode);
    if(!stat(path,&st) && st.st_size>=1024*1024)rename(path,old);
    return open(path,O_WRONLY|O_CREAT|O_APPEND|O_NOFOLLOW,0600);
}
static void logline(const char *mode,const void *buf,size_t len) {
    int fd=logfd(mode);if(fd>=0){(void)write(fd,buf,len);close(fd);}
}
int main(int argc,char **argv) {
    if(geteuid()!=0 || argc!=3 || !valid(argv[2]))return 2;
    const char *mode=argv[2];char pidpath[256],lockpath[256],childpath[256];
    snprintf(pidpath,sizeof pidpath,"%s/run/%s.supervisor.pid",base,mode);
    snprintf(childpath,sizeof childpath,"%s/run/%s.child.pid",base,mode);
    snprintf(lockpath,sizeof lockpath,"%s/run/%s.supervisor.lock",base,mode);
    if(!strcmp(argv[1],"stop")) {
        FILE *p=fopen(pidpath,"r");long pid=0;if(p){(void)fscanf(p,"%ld",&pid);fclose(p);}
        if(pid>1 && same_process((pid_t)pid)) {
            kill((pid_t)pid,SIGTERM);
            for(int i=0;i<100 && same_process((pid_t)pid);i++)usleep(100000);
            if(same_process((pid_t)pid))return 1;
        }
        return 0;
    }
    if(strcmp(argv[1],"run"))return 2;
    int lock=open(lockpath,O_CREAT|O_RDWR|O_NOFOLLOW,0600);if(lock<0)return 1;
    if(flock(lock,LOCK_EX|LOCK_NB))return errno==EWOULDBLOCK?0:1;
    FILE *p=fopen(pidpath,"w");if(!p)return 1;fprintf(p,"%ld\n",(long)getpid());fclose(p);
    struct sigaction action={0};action.sa_handler=stop;sigemptyset(&action.sa_mask);
    sigaction(SIGTERM,&action,NULL);sigaction(SIGINT,&action,NULL);signal(SIGPIPE,SIG_IGN);
    unsigned delay=5;
    while(!stopping) {
        int output[2];if(pipe(output))break;
        time_t began=time(NULL);pid_t child=fork();if(child<0)break;
        if(!child) {
            setsid();close(output[0]);dup2(output[1],STDOUT_FILENO);dup2(output[1],STDERR_FILENO);close(output[1]);
            int null=open("/dev/null",O_RDONLY);if(null>=0){dup2(null,STDIN_FILENO);if(null>2)close(null);}
            execl("/volume1/dorm-menu/native/bin/dorm-launcher","dorm-launcher",mode,NULL);_exit(127);
        }
        close(output[1]);p=fopen(childpath,"w");if(p){fprintf(p,"%ld\n",(long)child);fclose(p);}
        int status=0,finished=0,sent=0;time_t terminated=0;
        while(!finished) {
            if(stopping && !sent){kill(-child,SIGTERM);sent=1;terminated=time(NULL);}
            if(sent && time(NULL)-terminated>=3)kill(-child,SIGKILL);
            fd_set ready;FD_ZERO(&ready);FD_SET(output[0],&ready);struct timeval timeout={1,0};
            if(select(output[0]+1,&ready,NULL,NULL,&timeout)>0) {
                char data[8192];ssize_t n=read(output[0],data,sizeof data);if(n>0)logline(mode,data,(size_t)n);
            }
            finished=waitpid(child,&status,WNOHANG)==child;
        }
        close(output[0]);unlink(childpath);
        char message[128];int n=snprintf(message,sizeof message,"[supervisor] child exit=%d signal=%d\n",WIFEXITED(status)?WEXITSTATUS(status):-1,WIFSIGNALED(status)?WTERMSIG(status):0);
        logline(mode,message,(size_t)n);
        if(time(NULL)-began>=60)delay=5;
        for(unsigned i=0;i<delay && !stopping;i++)sleep(1);
        if(delay<60)delay=delay*2>60?60:delay*2;
    }
    unlink(pidpath);unlink(childpath);close(lock);return 0;
}
