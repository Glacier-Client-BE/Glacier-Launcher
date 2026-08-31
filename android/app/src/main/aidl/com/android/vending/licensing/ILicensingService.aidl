package com.android.vending.licensing;

import com.android.vending.licensing.ILicenseResultListener;

// The public AIDL contract for the Play Store's licensing Binder service,
// as documented in Google's own (now-archived) Play Licensing Library (LVL)
// sample — see PlayLicensing.kt for why this is hand-written here instead
// of a vendored copy of that sample. The package/interface/method names and
// signature below are load-bearing: they must match exactly what the
// com.android.vending package on-device implements, since AIDL binds by
// interface descriptor, not by source origin.
interface ILicensingService {
    void checkLicense(long nonce, String packageName, ILicenseResultListener listener);
}
