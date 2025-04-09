const fallbackCopyTextToClipboard = (text, onSuccess = false, onError = false) => {
    const textArea = document.createElement("textarea")
    textArea.value = text;

    textArea.style.top = "0";
    textArea.style.left = "0";
    textArea.style.position = "fixed";
  
    document.body.appendChild(textArea);
    textArea.focus();
    textArea.select();
  
    try {
      const successful = document.execCommand('copy')
      const msg = successful ? 'successful' : 'unsuccessful'
      onSuccess()
    } catch (err) {
      onError()
    }
  
    document.body.removeChild(textArea);
  }
  export const copyTextToClipboard = (text, onSuccess, onError) => {
    if (!navigator.clipboard) {
      fallbackCopyTextToClipboard(text, onSuccess, onError)
      return;
    }
    navigator.clipboard.writeText(text).then(function() {
      onSuccess()
    }, function(err) {
      onError()
    });
  }
