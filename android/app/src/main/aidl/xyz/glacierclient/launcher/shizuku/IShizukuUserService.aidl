package xyz.glacierclient.launcher.shizuku;

// AIDL contract for the privileged remote service Shizuku runs on this
// app's behalf (Shizuku.bindUserService) — the official, documented way to
// run app code with Shizuku's elevated (adb shell / rooted-manager) UID,
// as opposed to unofficial reflection into Shizuku's internals. See
// ShizukuExecutor.kt / ShizukuUserService.kt.
interface IShizukuUserService {
    // Runs [command] with Runtime.exec from inside the elevated process and
    // waits for it to finish, returning "<exitCode>\n<stdout>\n---STDERR---\n<stderr>".
    String exec(in String[] command);

    void destroy() = 16777114; // Shizuku's reserved "destroy" transaction code.
}
