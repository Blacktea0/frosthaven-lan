using System;
using System.Diagnostics;
using System.IO.MemoryMappedFiles;
using System.Text;
using FrosthavenLan;
class SessionTests {
 static void Write(MemoryMappedViewAccessor view,int slot,string text) {
  byte[] bytes=Encoding.UTF8.GetBytes(text);view.Write(slot,1);view.WriteArray(slot+8,bytes,0,bytes.Length);view.Write(slot+4,bytes.Length);view.Write(slot,2);
 }
 static string Read(MemoryMappedViewAccessor view,int slot) {
  int count=view.ReadInt32(slot+4);byte[] bytes=new byte[count];view.ReadArray(slot+8,bytes,0,count);return Encoding.UTF8.GetString(bytes);
 }
 static void Require(bool success,string message) {if(!success)throw new Exception(message);Console.WriteLine("PASS: "+message);}
 static int Main() {
  try {
   using(var mapping=MemoryMappedFile.CreateNew("Local\\FrosthavenLan-"+Process.GetCurrentProcess().Id,65536))
   using(var view=mapping.CreateViewAccessor()) {
    Session.Report("BACKEND FFSNet.Backend.EnetNetworkBackend main-thread=1");
    Session.Report("IP input enabled");
    Require(Read(view,0)=="READY","readiness is retained after unrelated diagnostic events");
    Session.Report("PUMP ERROR 网络组件测试失败");
    Require(Read(view,0).Contains("网络组件测试失败"),"Unicode errors are transmitted without files");
    Write(view,16384,"request-1|status");
    int calls=0;
    Action<string> command=s=>{calls++;Session.Report("STATUS 后端="+s);};
    Session.Pump(command);Session.Pump(command);
    Require(calls==1,"command requests execute once on the pump thread");
    Require(Read(view,32768)=="request-1|STATUS 后端=status"+Environment.NewLine,"command responses preserve correlation and Unicode");
   }
   return 0;
  } catch(Exception e){Console.Error.WriteLine(e);return 1;}
 }
}
