using System;
using System.Diagnostics;
using System.Linq;
using System.Reflection;
using System.Threading;
namespace FrosthavenLan {
 internal static class RuntimeAssemblies {
  internal static Assembly Find(string name) {
   return AppDomain.CurrentDomain.GetAssemblies().FirstOrDefault(a=>a.GetName().Name==name);
  }
  internal static Assembly Require(string name) {
   var assembly=Find(name);
   if(assembly==null)throw new InvalidOperationException("Required runtime assembly is not loaded: "+name);
   return assembly;
  }
  internal static void WaitFor(string[] names,int timeoutMilliseconds,Action<string> log) {
   var timer=Stopwatch.StartNew();
   string previous=null;
   while(true) {
    string missing=String.Join(", ",names.Where(name=>Find(name)==null).ToArray());
    if(missing.Length==0)return;
    if(missing!=previous){log("WAITING for runtime assemblies: "+missing);previous=missing;}
    if(timer.ElapsedMilliseconds>=timeoutMilliseconds)throw new TimeoutException("Runtime assembly initialization timed out: "+missing);
    Thread.Sleep(50);
   }
  }
 }
}
