using System;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using System.Threading;
using FrosthavenLan;
class RuntimeAssembliesTests {
 static int Main() {
  try {
   var messages=new List<string>();
   var waiting=new ManualResetEvent(false);
   var loader=new Thread(()=>{
    waiting.WaitOne();
    AppDomain.CurrentDomain.DefineDynamicAssembly(new AssemblyName("Assembly-CSharp"),AssemblyBuilderAccess.Run);
    AppDomain.CurrentDomain.DefineDynamicAssembly(new AssemblyName("UnityEngine.CoreModule"),AssemblyBuilderAccess.Run);
   });
   loader.Start();
   RuntimeAssemblies.WaitFor(new[]{"Assembly-CSharp","UnityEngine.CoreModule"},5000,s=>{messages.Add(s);waiting.Set();});
   loader.Join();
   if(messages.Count==0)throw new Exception("The cold-start wait was not exercised");
   RuntimeAssemblies.Require("Assembly-CSharp");RuntimeAssemblies.Require("UnityEngine.CoreModule");
   Console.WriteLine("PASS: missing Unity/game assemblies are awaited until loaded");
   RuntimeAssemblies.WaitFor(new[]{"Assembly-CSharp","UnityEngine.CoreModule"},0,s=>{throw new Exception("Already loaded assemblies must not wait");});
   Console.WriteLine("PASS: already loaded assemblies continue immediately");
   try { RuntimeAssemblies.WaitFor(new[]{"Missing.Test.Assembly"},0,s=>{});throw new Exception("Expected timeout"); }
   catch(TimeoutException e){if(!e.Message.Contains("Missing.Test.Assembly"))throw;}
   Console.WriteLine("PASS: timeout identifies missing assemblies");
   return 0;
  }catch(Exception e){Console.Error.WriteLine(e);return 1;}
 }
}
