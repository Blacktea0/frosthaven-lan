using System;
using System.Diagnostics;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
namespace FrosthavenLan {
 public static class DiagnosticTrace {
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern IntPtr CreateFile(string name,uint access,uint share,IntPtr attributes,uint disposition,uint flags,IntPtr template);
  [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool WaitNamedPipe(string name,uint timeout);
  [DllImport("kernel32.dll",SetLastError=true)] static extern bool WriteFile(IntPtr handle,IntPtr buffer,uint count,out uint written,IntPtr overlapped);
  [DllImport("kernel32.dll",SetLastError=true)] static extern bool ReadFile(IntPtr handle,byte[] buffer,uint count,out uint read,IntPtr overlapped);
  [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
  static void WriteAll(IntPtr handle,byte[] bytes) {
   var pinned=GCHandle.Alloc(bytes,GCHandleType.Pinned);
   try {
    int offset=0;
    while(offset<bytes.Length){
     uint written;
     if(!WriteFile(handle,IntPtr.Add(pinned.AddrOfPinnedObject(),offset),(uint)Math.Min(bytes.Length-offset,65536),out written,IntPtr.Zero)||written==0)
      throw new IOException("Diagnostic pipe write failed: "+Marshal.GetLastWin32Error());
     offset+=(int)written;
    }
   }finally{pinned.Free();}
  }
  public static void Write(string message) {
   // Optional once the launcher exits. Use Win32 I/O so diagnostics do not depend
   // on the embedded Mono runtime's FileStream support for named pipes.
   IntPtr handle=new IntPtr(-1);
   try {
    string name=@"\\.\pipe\FrosthavenLan-Diagnostics-"+Process.GetCurrentProcess().Id;
    handle=CreateFile(name,0xc0000000,0,IntPtr.Zero,3,0,IntPtr.Zero);
    if(handle==new IntPtr(-1)){
     if(Marshal.GetLastWin32Error()!=231||!WaitNamedPipe(name,1000))return;
     handle=CreateFile(name,0xc0000000,0,IntPtr.Zero,3,0,IntPtr.Zero);
     if(handle==new IntPtr(-1))return;
    }
    byte[] bytes=Encoding.UTF8.GetBytes(message);
    WriteAll(handle,BitConverter.GetBytes((long)bytes.Length));WriteAll(handle,bytes);
    byte[] ack=new byte[1];uint read;
    if(!ReadFile(handle,ack,1,out read,IntPtr.Zero)||read!=1||ack[0]!=1)
     throw new IOException("Diagnostic record was not acknowledged");
   }catch(Exception){/* Shared-memory readiness/errors remain available. */}
   finally{if(handle!=new IntPtr(-1))CloseHandle(handle);}
  }
 }
}
