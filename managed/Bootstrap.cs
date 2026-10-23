using System;
using System.IO;
using System.Reflection;
namespace FrosthavenLan {
 public static class Bootstrap {
  public static void Start() {
   DiagnosticTrace.Write("Bootstrap.Start entered");
   string dir=Path.GetDirectoryName(typeof(Bootstrap).Assembly.Location);
   AppDomain.CurrentDomain.AssemblyResolve += (s,e) => { string f=Path.Combine(dir,new AssemblyName(e.Name).Name+".dll"); DiagnosticTrace.Write("Resolving managed dependency: "+f+"; exists="+File.Exists(f)); return File.Exists(f)?Assembly.LoadFrom(f):null; };
   try { Assembly.LoadFrom(Path.Combine(dir,"LanPayload.dll")).GetType("FrosthavenLan.Payload").GetMethod("Install").Invoke(null,null); }
   catch(Exception e) { Session.Report("BOOTSTRAP ERROR "+e); }
  }
 }
}
