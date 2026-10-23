using System;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;
namespace FrosthavenLan {
 public static class LanChannels {
  public static void Install(Harmony harmony,Type backend){
   var join=AccessTools.Method(backend,"JoinAsync");
   if(join==null)throw new MissingMethodException(backend.FullName,"JoinAsync");
   harmony.Patch(join,transpiler:new HarmonyMethod(typeof(LanChannels),"ConnectWithAvatarChannel"));
  }
  public static IEnumerable<CodeInstruction> ConnectWithAvatarChannel(IEnumerable<CodeInstruction> code){
   int count=0;
   foreach(var instruction in code){
    var method=instruction.operand as MethodInfo;
    if((instruction.opcode==OpCodes.Call||instruction.opcode==OpCodes.Callvirt)&&method!=null&&method.DeclaringType.FullName=="ENet.Host"&&method.Name=="Connect"&&method.GetParameters().Length==1){
     var overload=method.DeclaringType.GetMethod("Connect",new[]{method.GetParameters()[0].ParameterType,typeof(int)});
     if(overload==null)throw new MissingMethodException(method.DeclaringType.FullName,"Connect(Address, int)");
     // ENet normalizes the original default count (0) to one channel.
     // Main uses channel 0; player avatar image fragments use channel 1.
     var channels=new CodeInstruction(OpCodes.Ldc_I4_2);
     channels.labels.AddRange(instruction.labels);instruction.labels.Clear();
     channels.blocks.AddRange(instruction.blocks);instruction.blocks.Clear();
     yield return channels;
     instruction.operand=overload;count++;
    }
    yield return instruction;
   }
   if(count!=1)throw new Exception("Expected exactly one ENet Connect(Address) call, found "+count);
  }
 }
}
