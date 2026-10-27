using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using System.Reflection.Emit;
using HarmonyLib;
namespace FrosthavenLan {
 // Inspect managed IL without invoking game methods. Keep discovery scoped to
 // known owners; never pick the first vaguely similar method from the assembly.
 public static class PatchDiscovery {
  const BindingFlags All=BindingFlags.Public|BindingFlags.NonPublic|BindingFlags.Static|BindingFlags.Instance|BindingFlags.DeclaredOnly;
  static readonly Dictionary<short,OpCode> Opcodes=typeof(OpCodes).GetFields().Where(f=>f.FieldType==typeof(OpCode)).Select(f=>(OpCode)f.GetValue(null)).ToDictionary(o=>o.Value);
  public static string Describe(MethodBase method){return method.DeclaringType.FullName+"::"+method.Name+" [0x"+method.MetadataToken.ToString("x8")+"]";}
  static IEnumerable<Type> Scope(Type owner){yield return owner;foreach(var child in owner.GetNestedTypes(All))foreach(var type in Scope(child))yield return type;}
  static IEnumerable<MethodInfo> Methods(Type owner){return Scope(owner).SelectMany(t=>t.GetMethods(All)).Where(m=>!m.ContainsGenericParameters&&m.GetMethodBody()!=null);}
  static MethodInfo Unique(string purpose,IEnumerable<MethodInfo> candidates){
   var matches=candidates.ToArray();
   if(matches.Length!=1)throw new NotSupportedException("Compatibility check failed: "+purpose+"; expected exactly one structural match, found "+matches.Length+". Candidates: "+String.Join("; ",matches.Select(Describe).ToArray()));
   return matches[0];
  }
  public static List<CodeInstruction> Read(MethodBase method){
   var body=method.GetMethodBody();if(body==null)throw new NotSupportedException("No managed IL: "+Describe(method));
   var bytes=body.GetILAsByteArray();var result=new List<CodeInstruction>();
   var typeArgs=method.DeclaringType.GetGenericArguments();var methodArgs=method.IsGenericMethod?method.GetGenericArguments():null;
   int offset=0;
   while(offset<bytes.Length){
    int first=bytes[offset++];if(first==0xfe){Need(bytes,offset,1);first=0xfe00|bytes[offset++];}
    OpCode opcode;if(!Opcodes.TryGetValue(unchecked((short)first),out opcode))throw new InvalidDataException("Unknown IL opcode in "+Describe(method));
    object operand=null;int size;
    switch(opcode.OperandType){
     case OperandType.InlineNone:size=0;break;
     case OperandType.ShortInlineI:case OperandType.ShortInlineVar:case OperandType.ShortInlineBrTarget:size=1;break;
     case OperandType.InlineVar:size=2;break;
     case OperandType.InlineI8:case OperandType.InlineR:size=8;break;
     case OperandType.InlineSwitch:
      Need(bytes,offset,4);int count=BitConverter.ToInt32(bytes,offset);
      if(count<0||count>(bytes.Length-offset-4)/4)throw new InvalidDataException("Invalid IL switch in "+Describe(method));
      size=4+count*4;break;
     case OperandType.InlineField:case OperandType.InlineMethod:case OperandType.InlineTok:case OperandType.InlineType:
     case OperandType.InlineString:case OperandType.InlineSig:case OperandType.InlineBrTarget:case OperandType.InlineI:case OperandType.ShortInlineR:size=4;break;
     default:throw new InvalidDataException("Unsupported IL operand in "+Describe(method));
    }
    Need(bytes,offset,size);
    if(opcode.OperandType==OperandType.InlineMethod||opcode.OperandType==OperandType.InlineField)
     operand=method.Module.ResolveMember(BitConverter.ToInt32(bytes,offset),typeArgs,methodArgs);
    else if(opcode.OperandType==OperandType.InlineI)operand=BitConverter.ToInt32(bytes,offset);
    else if(opcode.OperandType==OperandType.ShortInlineI)operand=(sbyte)bytes[offset];
    result.Add(new CodeInstruction(opcode,operand));offset+=size;
   }
   return result;
  }
  static void Need(byte[] bytes,int offset,int count){if(count<0||count>bytes.Length-offset)throw new InvalidDataException("Truncated managed IL");}
  static bool Constructor(CodeInstruction i,Type type){var ctor=i.operand as ConstructorInfo;return i.opcode==OpCodes.Newobj&&ctor!=null&&ctor.DeclaringType==type;}
  public static bool Calls(CodeInstruction i,string type,string name){var method=i.operand as MethodInfo;return (i.opcode==OpCodes.Call||i.opcode==OpCodes.Callvirt)&&method!=null&&method.DeclaringType.FullName==type&&method.Name==name;}
  public static ConstructorInfo Replacement(IList<CodeInstruction> code,Type eos,Type enet,FieldInfo backend){
   var targets=code.Select((i,n)=>new {i,n}).Where(x=>Constructor(x.i,eos)).ToArray();
   if(targets.Length!=1)throw new NotSupportedException("Expected exactly one EOS constructor, found "+targets.Length);
   int next=targets[0].n+1;while(next<code.Count&&code[next].opcode==OpCodes.Nop)next++;
   if(backend==null||!backend.IsStatic||!backend.FieldType.IsAssignableFrom(eos)||!backend.FieldType.IsAssignableFrom(enet)||next>=code.Count||code[next].opcode!=OpCodes.Stsfld||!Equals(code[next].operand,backend))
    throw new NotSupportedException("EOS constructor must directly initialize the compatible static backend field");
   var original=(ConstructorInfo)targets[0].i.operand;
   var signature=original.GetParameters().Select(p=>p.ParameterType).ToArray();
   // The default reflection binder accepts numeric widening (int -> long),
   // which is unsafe when replacing an existing IL call without conversions.
   var replacement=enet.GetConstructors().SingleOrDefault(c=>c.GetParameters().Select(p=>p.ParameterType).SequenceEqual(signature));
   if(replacement==null)throw new NotSupportedException("ENet constructor signature does not match EOS");
   return replacement;
  }
  public static MethodInfo BackendInitialization(Type net,Type eos,Type enet,FieldInfo backend){
   var target=Unique("EOS backend initialization",Methods(net).Where(m=>Read(m).Any(i=>Constructor(i,eos))));
   Replacement(Read(target),eos,enet,backend);return target;
  }
  public static IEnumerable<CodeInstruction> SwapBackend(IEnumerable<CodeInstruction> instructions,Type eos,Type enet,FieldInfo backend,MethodInfo ready,ILGenerator generator){
   var code=instructions.ToList();var replacement=Replacement(code,eos,enet,backend);
   foreach(var instruction in code){
    if(!Constructor(instruction,eos)){yield return instruction;continue;}
    if(instruction.blocks.Count!=0)throw new NotSupportedException("Unsupported exception boundary at backend constructor");
    // Until installation commits, preserve the game's original EOS creation.
    // Other patch failures must not leave a live ENet backend behind.
    var fallback=generator.DefineLabel();var done=generator.DefineLabel();
    var gate=new CodeInstruction(OpCodes.Call,ready);gate.labels.AddRange(instruction.labels);instruction.labels.Clear();
    yield return gate;yield return new CodeInstruction(OpCodes.Brfalse,fallback);
    yield return new CodeInstruction(OpCodes.Newobj,replacement);
    yield return new CodeInstruction(OpCodes.Castclass,backend.FieldType);
    yield return new CodeInstruction(OpCodes.Br,done);
    instruction.labels.Add(fallback);yield return instruction;
    yield return new CodeInstruction(OpCodes.Castclass,backend.FieldType);
    var merge=new CodeInstruction(OpCodes.Nop);merge.labels.Add(done);yield return merge;
   }
  }
  public static MethodInfo Connect(Type enet){
   var target=Unique("ENet connection",Methods(enet).Where(m=>Read(m).Any(i=>Calls(i,"ENet.Host","Connect"))));
   ValidateConnect(Read(target));return target;
  }
  public static void ValidateConnect(IList<CodeInstruction> code){
   var calls=code.Where(i=>Calls(i,"ENet.Host","Connect")).ToArray();
   if(calls.Length!=1)throw new NotSupportedException("Expected exactly one ENet Connect call, found "+calls.Length);
   var method=(MethodInfo)calls[0].operand;var parameters=method.GetParameters();
   if(method.IsStatic||parameters.Length!=1||parameters[0].ParameterType.FullName!="ENet.Address")throw new NotSupportedException("Unsupported ENet Connect signature");
   var overload=method.DeclaringType.GetMethods().SingleOrDefault(m=>m.Name=="Connect"&&m.GetParameters().Select(p=>p.ParameterType).SequenceEqual(new[]{parameters[0].ParameterType,typeof(int)}));
   if(overload==null||overload.IsStatic||overload.ReturnType!=method.ReturnType)throw new NotSupportedException("Incompatible ENet Connect(Address, int) overload");
  }
  public static FieldInfo AvatarOwner(MethodBase method){
   var owner=method.DeclaringType.DeclaringType;
   var fields=method.DeclaringType.GetFields(All).Where(f=>!f.IsStatic&&f.FieldType==owner).ToArray();
   if(owner==null||fields.Length!=1)throw new NotSupportedException("Expected exactly one avatar coroutine owner field: "+Describe(method));
   return fields[0];
  }
  public static MethodInfo Avatar(Type player){
   var target=Unique("avatar GPU copy coroutine",Methods(player).Where(m=>m.DeclaringType.DeclaringType==player&&!m.IsStatic&&m.GetParameters().Length==0&&m.ReturnType.FullName=="System.Boolean"&&Read(m).Any(i=>Calls(i,"UnityEngine.Graphics","Blit"))));
   ValidateAvatar(Read(target),target);return target;
  }
  public static void ValidateAvatar(IList<CodeInstruction> code,MethodBase method){
   var targets=code.Select((i,n)=>new {i,n}).Where(x=>Calls(x.i,"UnityEngine.RenderTexture","GetTemporary")).ToArray();
   if(targets.Length!=1)throw new NotSupportedException("Expected one avatar RenderTexture allocation");
   int start=targets[0].n-3;int end=-1;
   for(int n=targets[0].n;n<code.Count;n++)if(Calls(code[n],"UnityEngine.RenderTexture","ReleaseTemporary")){end=n;break;}
   if(start<0||end<0||code[start].opcode!=OpCodes.Ldc_I4||!Equals(code[start].operand,128)||code[start+1].opcode!=OpCodes.Ldc_I4||!Equals(code[start+1].operand,128)||code[start+2].opcode!=OpCodes.Ldc_I4_0)throw new NotSupportedException("Unsupported avatar resize block");
   if(code.Skip(start).Take(end-start+1).Count(i=>Calls(i,"UnityEngine.Graphics","Blit"))!=1)throw new NotSupportedException("Unsupported avatar GPU copy flow");
   AvatarOwner(method);
  }
 }
}
