using System;
using System.Reflection;
using System.Reflection.Emit;
using System.Threading;
using FrosthavenLan;
class RuntimeAssembliesTests {
 static int Main() {
  try {
   Exception failure=null;
   using(var done=new ManualResetEvent(false)) {
    var waiter=new Thread(()=>{
     try{RuntimeAssemblies.WaitFor(new[]{"Assembly-CSharp","UnityEngine.CoreModule"},5000);}
     catch(Exception e){failure=e;}
     finally{done.Set();}
    });
    waiter.Start();
    if(done.WaitOne(100))throw new Exception("The cold-start wait completed before the assemblies loaded");
    AppDomain.CurrentDomain.DefineDynamicAssembly(new AssemblyName("Assembly-CSharp"),AssemblyBuilderAccess.Run);
    AppDomain.CurrentDomain.DefineDynamicAssembly(new AssemblyName("UnityEngine.CoreModule"),AssemblyBuilderAccess.Run);
    if(!waiter.Join(6000))throw new Exception("The cold-start wait did not finish");
    if(failure!=null)throw failure;
   }
   RuntimeAssemblies.Require("Assembly-CSharp");RuntimeAssemblies.Require("UnityEngine.CoreModule");
   Console.WriteLine("PASS: missing Unity/game assemblies are awaited until loaded");
   RuntimeAssemblies.WaitFor(new[]{"Assembly-CSharp","UnityEngine.CoreModule"},0);
   Console.WriteLine("PASS: already loaded assemblies continue immediately");
   try { RuntimeAssemblies.WaitFor(new[]{"Missing.Test.Assembly"},0);throw new Exception("Expected timeout"); }
   catch(TimeoutException e){if(!e.Message.Contains("Missing.Test.Assembly"))throw;}
   Console.WriteLine("PASS: timeout identifies missing assemblies");
   return 0;
  }catch(Exception e){Console.Error.WriteLine(e);return 1;}
 }
}
