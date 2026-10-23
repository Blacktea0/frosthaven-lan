using System;
using System.Diagnostics;
using System.Threading;
using FrosthavenLan;
class SessionInterop {
 static int Main() {
  Console.WriteLine(Process.GetCurrentProcess().Id);
  Console.Out.Flush();
  if(Console.ReadLine()!="start")return 1;
  DiagnosticTrace.Write("MANAGED-LONG-BEGIN\n"+String.Concat(System.Linq.Enumerable.Repeat("异常调用栈\n",10000))+"\nMANAGED-LONG-END");
  try{try{throw new InvalidOperationException("inner failure");}catch(Exception inner){throw new Exception("outer failure",inner);}}
  catch(Exception e){DiagnosticTrace.Write("MANAGED ERROR "+e);}
  Session.Report("BACKEND FFSNet.Backend.EnetNetworkBackend");
  bool done=false;var timer=Stopwatch.StartNew();
  while(!done && timer.ElapsedMilliseconds<10000) {
   Session.Pump(command=>{Session.Report("网络组件："+command);done=command=="exit";});
   Thread.Sleep(10);
  }
  return done?0:1;
 }
}
