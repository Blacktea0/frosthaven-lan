using System;
using System.Collections.Generic;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;
namespace FrosthavenLan {
 public static class LanChannels {
  public static void Install(Harmony harmony,Type backend){
   Install(harmony,PatchDiscovery.Connect(backend));
  }
  public static void Install(Harmony harmony,MethodInfo join){
   if(PatchDiscovery.ConnectReplacement(PatchProcessor.GetOriginalInstructions(join))!=null)
    harmony.Patch(join,transpiler:new HarmonyMethod(typeof(LanChannels),"ConnectWithAvatarChannel"));
  }
  public static IEnumerable<CodeInstruction> ConnectWithAvatarChannel(IEnumerable<CodeInstruction> code){
   var instructions=new List<CodeInstruction>(code);
   var overload=PatchDiscovery.ConnectReplacement(instructions);
   foreach(var instruction in instructions){
    if(overload!=null&&PatchDiscovery.Calls(instruction,"ENet.Host","Connect")){
     // ENet normalizes the original default count (0) to one channel.
     // Main uses channel 0; player avatar image fragments use channel 1.
     var channels=new CodeInstruction(OpCodes.Ldc_I4_2);
     channels.labels.AddRange(instruction.labels);instruction.labels.Clear();
     channels.blocks.AddRange(instruction.blocks);instruction.blocks.Clear();
     yield return channels;
     instruction.operand=overload;
    }
    yield return instruction;
   }
  }
 }
}
