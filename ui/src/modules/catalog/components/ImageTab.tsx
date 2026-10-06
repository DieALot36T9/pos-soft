import { ImagePlus } from "lucide-react";
import { useTranslation } from "react-i18next";

import { pickImageFile } from "../../../shared/bridge/pickers";
import { useSave } from "../../../shared/bridge/useSave";
import { ActionButton } from "../../../shared/ui/ActionButton";
import { Alert } from "../../../shared/ui/Alert";
import { setProductImage } from "../api";

type ImageTabProps = {
  productId: string;
  name: string;
  // A data address from Rust, or null when there is no picture.
  image: string | null;
  onSaved: () => void;
};

// The product's picture (CAT-10). Rust reads, checks and resizes the file.
export function ImageTab({ productId, name, image, onSaved }: ImageTabProps) {
  const { t } = useTranslation("catalog");
  const save = useSave();

  const choose = () => {
    void pickImageFile(t("image.pickerTitle")).then((file) => {
      if (file) {
        save.run(() => setProductImage(productId, file), onSaved);
      }
    });
  };

  return (
    <div className="flex flex-col gap-3">
      <p className="text-text-muted">{t("image.intro")}</p>
      {image ? (
        <img
          src={image}
          alt={t("image.alt", { name })}
          className="size-64 rounded-ui border border-border bg-panel object-contain"
        />
      ) : (
        <p>{t("image.none")}</p>
      )}
      {save.problem ? <Alert kind="problem">{save.problem}</Alert> : null}
      {save.done ? <Alert kind="done">{t("image.saved")}</Alert> : null}
      <div>
        <ActionButton
          colour="blue"
          icon={ImagePlus}
          label={t("image.choose.label")}
          tip={t("image.choose.tip")}
          onClick={choose}
        />
      </div>
    </div>
  );
}
