using System;
using System.Diagnostics;
using System.IO.MemoryMappedFiles;
using System.Text;
using System.Threading;
namespace FrosthavenLan {
 public static class Session {
  static readonly MemoryMappedFile Mapping=MemoryMappedFile.OpenExisting("Local\\FrosthavenLan-"+Process.GetCurrentProcess().Id);
  static readonly MemoryMappedViewAccessor View=Mapping.CreateViewAccessor(0,65536);
  static readonly object Gate=new object();
  static string lastCommand;
  [ThreadStatic] static StringBuilder output;
  const int Startup=0, Request=16384, Response=32768;
  static void Write(int slot,string text) {
   byte[] bytes=Encoding.UTF8.GetBytes(text.Length>3000?text.Substring(0,3000):text);
   int seq=View.ReadInt32(slot)&~1;
   View.Write(slot,unchecked(seq+1));Thread.MemoryBarrier();
   View.WriteArray(slot+8,bytes,0,bytes.Length);View.Write(slot+4,bytes.Length);
   Thread.MemoryBarrier();View.Write(slot,unchecked(seq+2));
  }
  static string Read(int slot) {
   for(int i=0;i<8;i++) {
    int seq=View.ReadInt32(slot);if(seq==0 || (seq&1)!=0)continue;
    Thread.MemoryBarrier();int count=View.ReadInt32(slot+4);if(count<0||count>16376)return null;
    byte[] bytes=new byte[count];View.ReadArray(slot+8,bytes,0,count);Thread.MemoryBarrier();
    if(seq==View.ReadInt32(slot))return Encoding.UTF8.GetString(bytes);
   }return null;
  }
  public static void Report(string message) {
   DiagnosticTrace.Write(message);
   lock(Gate) {
    if(output!=null){output.AppendLine(message);return;}
    if(message.StartsWith("BACKEND FFSNet.Backend.EnetNetworkBackend"))Write(Startup,"READY");
    else if(message.Contains("ERROR")||message.StartsWith("REFUSED:"))Write(Startup,"ERROR "+message);
   }
  }
  public static void Pump(Action<string> command) {
   lock(Gate) {
    string request=Read(Request);if(String.IsNullOrEmpty(request)||request==lastCommand)return;
    lastCommand=request;int split=request.IndexOf('|');if(split<0)return;
    string id=request.Substring(0,split);output=new StringBuilder();
    try{command(request.Substring(split+1));Write(Response,id+"|"+output);}
    catch(Exception e){DiagnosticTrace.Write("COMMAND ERROR "+e);Write(Response,id+"|ERROR "+e);}
    finally{output=null;}
   }
  }
 }
}
