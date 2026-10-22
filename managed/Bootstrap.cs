using System;
using System.IO;
using System.Reflection;
namespace FrosthavenLan {
 public static class Bootstrap {
  public static void Start() {
   string dir=Path.GetDirectoryName(typeof(Bootstrap).Assembly.Location);
   AppDomain.CurrentDomain.AssemblyResolve += (s,e) => { string f=Path.Combine(dir,new AssemblyName(e.Name).Name+".dll"); return File.Exists(f)?Assembly.LoadFrom(f):null; };
   try { Assembly.LoadFrom(Path.Combine(dir,"LanPayload.dll")).GetType("FrosthavenLan.Payload").GetMethod("Install").Invoke(null,null); }
   catch(Exception e) { File.AppendAllText(Path.Combine(dir,"bootstrap.log"),e+Environment.NewLine); }
  }
 }
}
