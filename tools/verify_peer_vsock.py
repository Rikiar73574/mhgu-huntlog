#!/usr/bin/env python3
"""Test the recomp's unmodified virtual UDP backend, without a game/network.
Usage: verify_peer_vsock.py /path/to/MHGURecomp
Only the LDN link is replaced by an in-process datagram delivery stub.
"""
import subprocess,sys,tempfile
from pathlib import Path
source=Path(sys.argv[1]).resolve()/'runtime/net/vsock.c'
assert source.is_file()
code=r'''
#include <assert.h>
#include <sys/mman.h>
#include <stdio.h>
#include MHGU_VSOCK_SOURCE
static unsigned sent;
bool ldn_net_up(void){return true;}
uint32_t ldn_own_ip(void){return 0x0a020001;}
uint32_t ldn_broadcast_ip(void){return 0x0a0200ff;}
void netrate_udp(uint32_t len){(void)len;}
void ldn_send(const VPacket *p,const void *data,uint32_t len){sent++;vsock_deliver(p,data,len);}
static Cpu c;
static int create(void){c=(Cpu){0};c.r[0]=2;c.r[1]=2;x_socket(&c);assert((int)c.r[0]>=3);return (int)c.r[0];}
static void address(uint32_t at,uint8_t last,uint16_t port){uint8_t *a=G2H(at);memset(a,0,16);a[0]=16;a[1]=2;a[2]=port>>8;a[3]=port;a[4]=last==255?255:0;a[5]=a[4];a[6]=a[4];a[7]=last;}
static void bind_rx(int fd){address(0x10000,0,52689);c=(Cpu){0};c.r[0]=fd;c.r[1]=0x10000;c.r[2]=16;x_bind(&c);assert(c.r[0]==0);}
static void connect_tx(int fd){address(0x10000,255,52689);c=(Cpu){0};c.r[0]=fd;c.r[1]=0x10000;c.r[2]=16;x_connect(&c);assert(c.r[0]==0);}
static int receive(int fd){c=(Cpu){0};c.r[0]=fd;c.r[1]=0x14000;c.r[2]=4000;c.r[3]=0x80;x_recv(&c);return (int)c.r[0];}
int main(void){
 assert(mmap((void*)(GUEST_BASE+0x10000),0x10000,PROT_READ|PROT_WRITE,MAP_PRIVATE|MAP_ANONYMOUS|MAP_FIXED_NOREPLACE,-1,0)!=(void*)-1);
 int rx[2]={create(),create()},tx[2]={create(),create()};bind_rx(rx[0]);bind_rx(rx[1]);connect_tx(tx[0]);connect_tx(tx[1]);
 assert(receive(rx[0])==-1&&s_errno==11); // Empty reads must never block.
 for(unsigned round=0;round<2;round++){
  for(unsigned i=0;i<3605;i++)((uint8_t*)G2H(0x11000))[i]=(uint8_t)(i+round);
  c=(Cpu){0};c.r[0]=tx[round];c.r[1]=0x11000;c.r[2]=3605;c.r[3]=0x80;x_send(&c);assert(c.r[0]==3605);
  for(int peer=0;peer<2;peer++){assert(receive(rx[peer])==3605);assert(memcmp(G2H(0x11000),G2H(0x14000),3605)==0);assert(receive(rx[peer])==-1&&s_errno==11);}
  // A broadcast-connected RX would filter actual peer sources out. Our TX
  // is separate, so it deliberately has no usable incoming datagrams.
  assert(receive(tx[round])==-1&&s_errno==11);
 }
 assert(sent==2);
 for(int i=0;i<2;i++){c.r[0]=rx[i];x_close(&c);assert(c.r[0]==0);c.r[0]=tx[i];x_close(&c);assert(c.r[0]==0);}
 puts("Native recomp UDP passed: two endpoints, maximum packets, broadcast, unconnected RX, nonblocking empty reads, cleanup; no game or external network");
}
'''
with tempfile.TemporaryDirectory(prefix='huntlog-vsock-')as tmp:
 p=Path(tmp);(p/'test.c').write_text(code)
 subprocess.run(['clang','-D_GNU_SOURCE','-O2','-ffunction-sections','-fdata-sections','-Wl,--gc-sections','-pthread','-DMHGU_VSOCK_SOURCE="'+str(source)+'"',str(p/'test.c'),'-o',str(p/'test')],check=True)
 subprocess.run([str(p/'test')],check=True,timeout=5)
