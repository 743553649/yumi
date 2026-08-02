#!/usr/bin/env python3
import os
import sys
import glob
import shutil
import subprocess
import zipfile

APP_DIR = "/storage/emulated/0/yumi/android-app"
BUILD_DIR = os.path.join(APP_DIR, "build")
LIBS_DIR = os.path.join(BUILD_DIR, "libs")
CLASSES_DIR = os.path.join(BUILD_DIR, "classes")
DEX_DIR = os.path.join(BUILD_DIR, "dex")
GEN_DIR = os.path.join(BUILD_DIR, "gen")

ANDROID_JAR = "/data/data/com.termux/files/home/android-sdk/platforms/android-34/android.jar"
if not os.path.exists(ANDROID_JAR):
    ANDROID_JAR = "/data/data/com.termux/files/home/.android-sdk/platforms/android-34/android.jar"

GRADLE_CACHE = "/data/data/com.termux/files/home/.gradle/caches/modules-2/files-2.1"
KEYTOOL = "/data/data/com.termux/files/usr/lib/jvm/java-21-openjdk/bin/keytool"
JARSIGNER = "/data/data/com.termux/files/usr/lib/jvm/java-21-openjdk/bin/jarsigner"

MERGED_RES_DIR = os.path.join(BUILD_DIR, "res_merged")
os.makedirs(MERGED_RES_DIR, exist_ok=True)

# Copy app resources first
shutil.copytree(os.path.join(APP_DIR, "app/src/main/res"), MERGED_RES_DIR, dirs_exist_ok=True)

print("===> [1/7] Initializing build directories...")
shutil.rmtree(BUILD_DIR, ignore_errors=True)
os.makedirs(LIBS_DIR, exist_ok=True)
os.makedirs(CLASSES_DIR, exist_ok=True)
os.makedirs(DEX_DIR, exist_ok=True)
os.makedirs(GEN_DIR, exist_ok=True)
os.makedirs(MERGED_RES_DIR, exist_ok=True)

# Copy app resources first
shutil.copytree(os.path.join(APP_DIR, "app/src/main/res"), MERGED_RES_DIR, dirs_exist_ok=True)

print("===> [2/7] Extracting AAR and JAR dependencies from Gradle Cache...")
jar_paths = [ANDROID_JAR]

# Collect all jars and aars
counter = 0
aar_res_zips = []

EXCLUDED_KEYWORDS = [
    "bundletool", "aaptcompiler", "gradle-plugin",
    "com.android.tools.build", "kotlin-compiler",
    "kotlin-daemon", "kotlin-scripting", "kotlin-util-klib",
    "listenablefuture-1.0.jar"
]

ALLOWED_PATH_PATTERNS = [
    "aar_lib_",
    "androidx",
    "kotlin",
    "kyant"
]

for root, dirs, files in os.walk(GRADLE_CACHE):
    for f in files:
        full_path = os.path.join(root, f)
        if any(p in full_path for p in ALLOWED_PATH_PATTERNS):
            if f.endswith(".jar") and not f.endswith("-sources.jar") and not f.endswith("-javadoc.jar"):
                if not any(k in full_path for k in EXCLUDED_KEYWORDS):
                    jar_paths.append(full_path)
            elif f.endswith(".aar"):
                try:
                    with zipfile.ZipFile(full_path, 'r') as zip_ref:
                        if 'classes.jar' in zip_ref.namelist():
                            counter += 1
                            out_jar = os.path.join(LIBS_DIR, f"aar_lib_{counter}_{f[:-4]}.jar")
                            zip_ref.extract('classes.jar', LIBS_DIR)
                            os.rename(os.path.join(LIBS_DIR, 'classes.jar'), out_jar)
                            
                            # Strip stub R classes from AAR jar to prevent duplicate stub R class collisions
                            try:
                                with zipfile.ZipFile(out_jar, 'r') as j_in:
                                    j_files = j_in.namelist()
                                    non_r_files = [x for x in j_files if not (x.endswith('/R.class') or '/R$' in x or x.startswith('R$') or x == 'R.class')]
                                    if len(non_r_files) < len(j_files):
                                        temp_j = out_jar + ".tmp"
                                        with zipfile.ZipFile(temp_j, 'w') as j_out:
                                            for item in non_r_files:
                                                j_out.writestr(item, j_in.read(item))
                                        shutil.move(temp_j, out_jar)
                            except Exception as e_strip:
                                print(f"Warning stripping R classes from {out_jar}: {e_strip}")

                            jar_paths.append(out_jar)

                        
                        # Extract res/ entries from AAR and compile individually
                        res_entries = [name for name in zip_ref.namelist() if name.startswith("res/")]
                        if res_entries and ("poolingcontainer" in f or "activity" in f or "lifecycle" in f):
                            aar_target_res = os.path.join(BUILD_DIR, f"tmp_aar_res_{counter}")
                            for res_file in res_entries:
                                zip_ref.extract(res_file, aar_target_res)
                            res_dir = os.path.join(aar_target_res, "res")
                            if os.path.exists(res_dir):
                                out_zip = os.path.join(BUILD_DIR, f"res_aar_{counter}.zip")
                                cmd_c = ["aapt2", "compile", "--dir", res_dir, "-o", out_zip]
                                res_c = subprocess.run(cmd_c, capture_output=True, text=True)
                                if res_c.returncode == 0 and os.path.exists(out_zip):
                                    aar_res_zips.append(out_zip)

                except Exception as e:
                    print(f"Warning extracting {f}: {e}")


# Deduplicate jar_paths by artifact name
jar_dict = {}
for j in jar_paths:
    name = os.path.basename(j)
    import re
    base_key = re.sub(r'-\d+\.\d+.*\.jar$', '', name)
    jar_dict[base_key] = j

jar_paths = list(jar_dict.values())

# Strip stub R classes from ALL input jars before d8 to ensure no stub R class conflicts
def strip_r_from_jar(jpath):
    if not os.path.exists(jpath) or jpath == ANDROID_JAR:
        return jpath
    try:
        with zipfile.ZipFile(jpath, 'r') as j_in:
            j_files = j_in.namelist()
            r_files = [x for x in j_files if (x.endswith('/R.class') or '/R$' in x or x.startswith('R$') or x == 'R.class')]
            if r_files:
                clean_jar = os.path.join(LIBS_DIR, "clean_" + os.path.basename(jpath))
                with zipfile.ZipFile(clean_jar, 'w') as j_out:
                    for item in j_files:
                        if item not in r_files:
                            j_out.writestr(item, j_in.read(item))
                return clean_jar
    except Exception as e:
        print(f"Warning cleaning jar {jpath}: {e}")
    return jpath

clean_jar_paths = []
for j in jar_paths:
    clean_jar_paths.append(strip_r_from_jar(j))

jar_paths = clean_jar_paths

classpath_str = ":".join(jar_paths)


print("===> [3/7] Compiling Android resources with AAPT2...")
app_res_zip = os.path.join(BUILD_DIR, "res_app.zip")
cmd_aapt_compile = ["aapt2", "compile", "--dir", os.path.join(APP_DIR, "app/src/main/res"), "-o", app_res_zip]
subprocess.run(cmd_aapt_compile, check=True)

all_res_zips = [app_res_zip] + aar_res_zips

unaligned_apk = os.path.join(BUILD_DIR, "unaligned.apk")
manifest_file = os.path.join(APP_DIR, "app/src/main/AndroidManifest.xml")
extra_pkgs = [
    "androidx.customview.poolingcontainer",
    "androidx.compose.ui",
    "androidx.compose.material3",
    "androidx.compose.foundation",
    "androidx.compose.runtime",
    "androidx.compose.animation",
    "androidx.activity",
    "androidx.lifecycle",
    "androidx.lifecycle.runtime",
    "androidx.lifecycle.viewmodel",
    "androidx.appcompat",
    "androidx.core",
    "com.google.android.material"
]

cmd_aapt_link = [
    "aapt2", "link", "-I", ANDROID_JAR,
    "--manifest", manifest_file,
    "--min-sdk-version", "26",
    "--target-sdk-version", "34",
    "--auto-add-overlay",
    "--extra-packages", ":".join(extra_pkgs),
    "-o", unaligned_apk,
    "--java", GEN_DIR
] + all_res_zips
subprocess.run(cmd_aapt_link, check=True)

print("===> [4/7] Compiling Kotlin and Java sources...")
kt_sources = []
java_sources = []

# Collect generated R.java files
for root, dirs, files in os.walk(GEN_DIR):
    for f in files:
        if f.endswith(".java"):
            java_sources.append(os.path.join(root, f))

for root, dirs, files in os.walk(os.path.join(APP_DIR, "app/src/main/java")):
    for f in files:
        full_path = os.path.join(root, f)
        if f.endswith(".kt"):
            kt_sources.append(full_path)
        elif f.endswith(".java"):
            java_sources.append(full_path)


# Run kotlinc with Compose compiler plugin
cmd_kotlinc = [
    "kotlinc",
    "-Xplugin=/data/data/com.termux/files/usr/opt/kotlin/lib/compose-compiler-plugin.jar",
    "-cp", classpath_str,
    "-d", CLASSES_DIR,
    "-jvm-target", "17"
] + kt_sources
print(f"Compiling {len(kt_sources)} Kotlin files with Compose compiler plugin...")
res = subprocess.run(cmd_kotlinc, capture_output=True, text=True)


if res.returncode != 0:
    print("KOTLINC ERROR:\n", res.stderr)
    sys.exit(1)


# Run javac with compiled Kotlin classes in classpath
java_cp = classpath_str + ":" + CLASSES_DIR
cmd_javac = ["javac", "-cp", java_cp, "-d", CLASSES_DIR, "-source", "17", "-target", "17"] + java_sources
print(f"Compiling {len(java_sources)} Java files...")
subprocess.run(cmd_javac, check=True)

print("===> [5/7] Converting Bytecode to DEX (d8)...")
all_classes = []
for root, dirs, files in os.walk(CLASSES_DIR):
    for f in files:
        if f.endswith(".class"):
            all_classes.append(os.path.join(root, f))

# Also include dependency jars for d8
d8_inputs = all_classes + [j for j in jar_paths if j != ANDROID_JAR]
cmd_d8 = ["d8", "--lib", ANDROID_JAR, "--min-api", "26", "--output", DEX_DIR] + d8_inputs
print(f"Converting {len(all_classes)} classes and {len(d8_inputs) - len(all_classes)} jars with d8...")
res = subprocess.run(cmd_d8, capture_output=True, text=True)
if res.returncode != 0:
    print("D8 ERROR:\n", res.stderr)
    sys.exit(1)


print("===> [6/7] Packaging APK...")
final_apk = os.path.join(BUILD_DIR, "yumi-bridge.apk")
shutil.copyfile(unaligned_apk, final_apk)

dex_files = [f for f in os.listdir(DEX_DIR) if f.endswith(".dex")]
with zipfile.ZipFile(final_apk, 'a') as zf:
    for dex in dex_files:
        zf.write(os.path.join(DEX_DIR, dex), dex)

print("===> [7/7] Signing APK...")
keystore = os.path.join(BUILD_DIR, "debug.keystore")
if not os.path.exists(keystore):
    genkey_cmd = [
        KEYTOOL, "-genkey", "-v", "-keystore", keystore,
        "-storepass", "android", "-alias", "androiddebugkey",
        "-keypass", "android", "-keyalg", "RSA", "-keysize", "2048",
        "-validity", "10000", "-dname", "CN=Android Debug,O=Android,C=US"
    ]
    subprocess.run(genkey_cmd, check=True)

sign_cmd = [
    JARSIGNER, "-keystore", keystore, "-storepass", "android",
    "-keypass", "android", final_apk, "androiddebugkey"
]
subprocess.run(sign_cmd, check=True)

shutil.copyfile(final_apk, "/storage/emulated/0/yumi/yumi-bridge.apk")
shutil.copyfile(final_apk, "/storage/emulated/0/yumi/module/yumi-bridge.apk")

try:
    install_cmd = ["su", "-c", "cp /storage/emulated/0/yumi/yumi-bridge.apk /data/local/tmp/yumi-bridge.apk && pm install -r /data/local/tmp/yumi-bridge.apk"]
    res_inst = subprocess.run(install_cmd, capture_output=True, text=True)
    if res_inst.returncode == 0:
        print("📱 Automatically Installed/Updated APK on Device via Root PM!")
except Exception:
    pass

print("\n🎉 SUCCESS! APK Built and Signed Successfully!")
print(f"Output: /storage/emulated/0/yumi/yumi-bridge.apk")

