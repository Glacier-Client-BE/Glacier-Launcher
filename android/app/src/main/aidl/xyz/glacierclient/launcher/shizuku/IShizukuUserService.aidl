package xyz.glacierclient.launcher.shizuku;

// AIDL contract for the privileged remote service Shizuku runs on this
// app's behalf (Shizuku.bindUserService) — the official, documented way to
// run app code with Shizuku's elevated (adb shell / rooted-manager) UID,
// as opposed to unofficial reflection into Shizuku's internals. See
// ShizukuExecutor.kt / ShizukuUserService.kt.
interface IShizukuUserService {
    // AIDL requires either every method have an explicit transaction id or
    // none of them do — destroy() needs Shizuku's reserved code below, so
    // exec() needs one too now. 1 is what implicit auto-numbering would
    // have assigned it anyway (the first non-destroy method).
    //
    // Runs [command] with Runtime.exec from inside the elevated process and
    // waits for it to finish, returning "<exitCode>\n<stdout>\n---STDERR---\n<stderr>".
    String exec(in String[] command) = 1;

    void destroy() = 16777114; // Shizuku's reserved "destroy" transaction code.
}
