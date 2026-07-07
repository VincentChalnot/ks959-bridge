// SPDX-License-Identifier: GPL-2.0
/*
 * ks959_speed - Minimal kernel module to change Kingsun KS-959 IrDA dongle speed.
 *
 * The Linux kernel's usbfs check_ctrlrecip() blocks userspace control transfers
 * with USB_TYPE_CLASS + USB_RECIP_INTERFACE when wIndex doesn't match an existing
 * interface number. The KS-959 dongle uses wIndex=0x0001 as a protocol flag
 * (meaning "speed change"), but the device only has interface 0. This creates a
 * deadlock: the dongle needs bRequestType=0x21 (Class+Interface), but the kernel
 * rejects it because interface 1 doesn't exist.
 *
 * This module bypasses usbfs by calling usb_control_msg() directly from kernel
 * context. It matches the dongle by VID/PID, changes the speed in its probe()
 * function, then returns -ENODEV so it doesn't permanently claim the device.
 * The userspace bridge (ks959-bridge) can then open the dongle normally via nusb.
 *
 * Usage:
 *   sudo insmod ks959_speed.ko baud=115200
 *
 * The module can only be used once per USB plug cycle (returning -ENODEV prevents
 * re-probing until the device is physically reconnected).
 */

#include <linux/module.h>
#include <linux/moduleparam.h>
#include <linux/kernel.h>
#include <linux/usb.h>

#define KS959_VENDOR_ID   0x07d0
#define KS959_PRODUCT_ID  0x4959
#define KS959_REQ_SEND    0x09

/* Default baud rate: 115200 (what the Cressi Donatello uses). */
static unsigned int baud = 115200;
module_param(baud, uint, 0444);
MODULE_PARM_DESC(baud, "Desired IrDA link baud rate (default: 115200)");

/*
 * Speed-change payload (8 bytes, packed, little-endian):
 *   [baudrate_le32] [flags=0x03] [reserved=0,0,0]
 * flags = 0x03 means 8 data bits.
 */
static int ks959_change_speed(struct usb_device *udev, unsigned int baud_rate)
{
	u8 payload[8];
	int ret;

	memset(payload, 0, sizeof(payload));
	payload[0] = (u8)(baud_rate & 0xff);
	payload[1] = (u8)((baud_rate >> 8) & 0xff);
	payload[2] = (u8)((baud_rate >> 16) & 0xff);
	payload[3] = (u8)((baud_rate >> 24) & 0xff);
	payload[4] = 0x03; /* KS_DATA_8_BITS */

	/*
	 * bRequestType: USB_DIR_OUT | USB_TYPE_CLASS | USB_RECIP_INTERFACE = 0x21
	 * bRequest:     0x09 (KINGSUN_REQ_SEND)
	 * wValue:       0x0200 (speed change identifier)
	 * wIndex:       0x0001 (protocol flag: "this is a speed change")
	 * wLength:      8
	 */
	ret = usb_control_msg(udev,
			      usb_sndctrlpipe(udev, 0),
			      KS959_REQ_SEND,
			      USB_DIR_OUT | USB_TYPE_CLASS | USB_RECIP_INTERFACE,
			      0x0200, 0x0001,
			      payload, sizeof(payload),
			      1000 /* timeout ms */);

	if (ret < 0)
		return ret;
	if (ret != sizeof(payload))
		return -EIO;

	return 0;
}

static int ks959_speed_probe(struct usb_interface *interface,
			     const struct usb_device_id *id)
{
	struct usb_device *udev = interface_to_usbdev(interface);
	int ret;

	ret = ks959_change_speed(udev, baud);
	if (ret) {
		dev_err(&interface->dev,
			"ks959_speed: speed change to %u baud failed: %d\n",
			baud, ret);
		return ret;
	}

	dev_info(&interface->dev,
		 "ks959_speed: dongle speed changed to %u baud\n", baud);

	/*
	 * Return -ENODEV so we don't permanently claim the device.
	 * The dongle retains the speed setting. The userspace bridge
	 * (ks959-bridge) will open the dongle via nusb afterward.
	 *
	 * Caveat: this driver won't re-probe until the device is physically
	 * unplugged and replugged. That's fine — we only need one speed
	 * change per session.
	 */
	return -ENODEV;
}

static void ks959_speed_disconnect(struct usb_interface *interface)
{
	/* Never called because probe returns -ENODEV. */
}

static const struct usb_device_id ks959_speed_table[] = {
	{ USB_DEVICE(KS959_VENDOR_ID, KS959_PRODUCT_ID) },
	{ } /* terminator */
};
MODULE_DEVICE_TABLE(usb, ks959_speed_table);

static struct usb_driver ks959_speed_driver = {
	.name       = "ks959_speed",
	.probe      = ks959_speed_probe,
	.disconnect = ks959_speed_disconnect,
	.id_table   = ks959_speed_table,
};

module_usb_driver(ks959_speed_driver);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("ks959-bridge project");
MODULE_DESCRIPTION("Bypass usbfs check_ctrlrecip to change Kingsun KS-959 IrDA dongle speed");
