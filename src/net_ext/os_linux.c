#include <assert.h>
#include <stdint.h>
#include <string.h>
#include <unistd.h>

#include <sys/socket.h>
#include <linux/if.h>
#include <linux/if_tun.h>
#include <sys/ioctl.h>
#include <errno.h>
#include <stdio.h>
#include <poll.h>
#include <fcntl.h>
#include <linux/if_packet.h>
#include <signal.h>
#include <time.h>

/**
 * fd ‒ the fd to turn into TUN or TAP.
 * name ‒ the name to use. If empty, kernel will assign something by itself.
 *   Must be buffer with capacity at least 33.
 * mode ‒ 1 = TUN, 2 = TAP.
 * packet_info ‒ if packet info should be provided, if the given value is 0 it will not prepend packet info.
 */
int tuntap_setup(int fd, unsigned char *name, int mode, int packet_info) {
	struct ifreq ifr;
	memset(&ifr, 0, sizeof ifr);
	switch (mode) {
		case 1:
			ifr.ifr_flags = IFF_TUN;
			break;
		case 2:
			ifr.ifr_flags = IFF_TAP;
			break;
		default:
			assert(0);
	}
	ifr.ifr_flags |= (IFF_UP | IFF_RUNNING);
	// If no packet info needs to be provided add corresponding flag
	if (!packet_info) {
		ifr.ifr_flags |= IFF_NO_PI;
	}

	// Leave one for terminating '\0'. No idea if it is needed, didn't find
	// it in the docs, but assuming the worst.
	strncpy(ifr.ifr_name, (char *)name, IFNAMSIZ - 1);

	int ioresult = ioctl(fd, TUNSETIFF, &ifr);
	if (ioresult < 0) {
		return errno;
	}
	fcntl(fd,F_SETFL,O_NONBLOCK);
    
	strncpy((char *)name, ifr.ifr_name, IFNAMSIZ < 32 ? IFNAMSIZ : 32);
	name[32] = '\0';
	return 0;
}



int setSocketRecvBuf(int fd,int bufsize) {
	return setsockopt(fd,SOL_SOCKET,SO_RCVBUF,&bufsize,sizeof(bufsize));
}

int setSocketSendBuf(int fd,int bufsize) {
	return setsockopt(fd,SOL_SOCKET,SO_SNDBUF,&bufsize,sizeof(bufsize));
}

int send_udp_packet(int fd,const void *buf,int len, void *addr,int addr_len) {
	
	int ret = sendto(fd,buf,len,0,addr,addr_len);
	if (ret!=0) {
		printf("error send msg,ret=%d,err=%d\n",ret,errno);
	}
	return ret;
}

//等待一个文件的读事件，返回0表示超时，1表示成功，负值表示失败
int c_wait_single_fd_read(int fd,int timeout_msec) {
	struct pollfd ev_poll;
	ev_poll.fd = fd;
	ev_poll.events = POLLIN;
	int ret = poll(&ev_poll,1,timeout_msec);
	return ret;

}

///读取接口的MAC地址
int c_get_if_mac(unsigned char *name, unsigned char *mac) {
	int sock = socket(AF_PACKET,SOCK_RAW,0);

	struct ifreq ifr;
	memset(&ifr, 0, sizeof ifr);	
	strncpy(ifr.ifr_name, (char *)name, IFNAMSIZ - 1);

	int ioresult = ioctl(sock, SIOCGIFHWADDR, &ifr);
	close(sock);
	if (ioresult < 0) {
		printf("error=%d,name=%s\n",ioresult,ifr.ifr_name);
		return errno;
	}	
	memcpy(mac,&ifr.ifr_hwaddr.sa_data,6);

	return 0;
}

///设置接口的MAC地址
int c_set_if_mac(unsigned char *name, unsigned char *mac) {
	int sock = socket(AF_PACKET,SOCK_RAW,0);

	struct ifreq ifr;
	memset(&ifr, 0, sizeof ifr);	
	strncpy(ifr.ifr_name, (char *)name, IFNAMSIZ - 1);
	memcpy(&ifr.ifr_hwaddr.sa_data,mac,6);
	int ioresult = ioctl(sock, SIOCSIFHWADDR, &ifr);
	close(sock);
	if (ioresult < 0) {
		printf("error=%d,name=%s\n",ioresult,ifr.ifr_name);
		return errno;
	}	
	
	return 0;
}

///读取接口的MTU
int c_get_if_mtu(unsigned char *name, int *mtu) {
	int sock = socket(AF_PACKET,SOCK_RAW,0);

	struct ifreq ifr;
	memset(&ifr, 0, sizeof ifr);	
	strncpy(ifr.ifr_name, (char *)name, IFNAMSIZ - 1);

	int ioresult = ioctl(sock, SIOCGIFMTU, &ifr);
	if (ioresult < 0) {
		return errno;
	}
	*mtu = ifr.ifr_mtu;
	return 0;
}


///设置接口的IP地址
int c_set_if_ip(unsigned char *name, struct sockaddr *ip,struct sockaddr * mask) {
	int sock = socket(AF_PACKET,SOCK_RAW,0);

	struct ifreq ifr;
	memset(&ifr, 0, sizeof ifr);	
	strncpy(ifr.ifr_name, (char *)name, IFNAMSIZ - 1);
	memcpy(&ifr.ifr_addr,ip,sizeof(struct sockaddr));
	int ioresult = ioctl(sock, SIOCSIFADDR, &ifr);
	
	if (ioresult < 0) {
		close(sock);
		printf("set ip address error=%d,name=%s\n",ioresult,ifr.ifr_name);
		return errno;
	}

	memcpy(&ifr.ifr_netmask,mask,sizeof(struct sockaddr));
	ioresult = ioctl(sock, SIOCSIFNETMASK, &ifr);
	close(sock);
	if (ioresult < 0) {
		printf("set ip netmask error=%d,name=%s\n",ioresult,ifr.ifr_name);
		return errno;
	}
	return 0;
}

typedef int (*timer_callback)(sigval_t);

int c_create_timer(void *ptr_data,timer_callback call_back,timer_t *timer_id) {
	struct sigevent sev;
    memset(&sev,0,sizeof(struct sigevent));

    sev.sigev_notify = SIGEV_THREAD;
    sev.sigev_notify_function = call_back;
    sev.sigev_value.sival_ptr = ptr_data;
    
    /* create timer */
    int res = timer_create(CLOCK_REALTIME, &sev, timer_id);
    if (res<0) {
        printf("error create time,ret=%d",res);
        return res;
    }
	return 0;
}
